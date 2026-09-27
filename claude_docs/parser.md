# Parser Crate — Navigation Map

PEG grammar (the `peg` crate) for `.sym` (symbols + rules) and `.pbl` (tasks).

**No line numbers here, on purpose** — see the note in `solver.md`. Grep the name.

## File Map

```
src/parser/
├── lib.rs          # Location, NodeData, Tree/Node aliases over trees::Tree<NodeData>
├── error.rs        # ParserError { loc, msg } + error_string(); PegError
├── grammar.rs      # the PEG itself: tokens → Tree<Token> with byte offsets
├── lang.rs         # the five entry points; wraps the grammar and annotates Locations
├── term.rs         # TermParser  — AST → TermBuf
├── rule.rs         # RuleParser  — AST → Vec<Rule> via RuleBuilder
├── symbol.rs       # SymbolParser — AST → SymbolProgram, including embedded Python
├── task.rs         # TaskParser  — AST → Task via TaskBuilder
├── py_term.rs      # the `Term` class injected into an embedded Python program
├── dir_loader.rs   # DirectoryParser — loads a tree of .sym/.pbl, collecting errors
├── syntax/         # the rewrite's grammar (precedence!) and `Expr`, not yet used by the loader
├── scope.rs        # TermScope — `Expr` → TermBuf, resolving names; `parser::term` goes through it
├── source.rs       # Source — spans back to Locations
├── legacy/         # test-only: verbatim copy of the pre-rewrite pipeline (grammar … task)
└── corpus_diff.rs  # test-only: the corpus must lower identically through legacy and current
```

## Entry points — `lang.rs`

| Function | Parses |
|----------|--------|
| `symbol(text)` | one `symbol` declaration |
| `terms(text)` | semicolon-separated terms |
| `lang_rule(text)` | one `rule` block |
| `task(text)` | one `task` block |
| `any(text)` | mixed content: tasks, rules, symbols |

Each returns `Result<_, ParserError>`, and `ParserError` carries the source
`Location` (row, col, len) so a message can point at the offending text.

## Key Types

### lib.rs
| Type | What it is |
|------|------------|
| `Location` | `{ col, row, len }` — where a node came from. |
| `NodeData` | `{ symbol, location }` — an AST node after annotation. |
| `Tree` / `Node` | `trees::Tree<NodeData>` / `trees::Node<NodeData>`. |

### the four parsers
| Type | What it is |
|------|------------|
| `TermParser` | `with_variables()` for tasks (bare identifiers become variables), `with_params()` for rules; `try_parse(&Node) -> TermBuf`. |
| `RuleParser` | Reads attributes, the `=>` / `<=>` body, then requirements. Helpers promote `find` targets and collect params out of the body. |
| `SymbolParser` | Name + attributes → `SymbolProgram`. A `py "…"` block runs at parse time and its `calculator` becomes the symbol's. |
| `TaskParser` | Goal, id, text, expected answers, conditions → `Task` through `TaskBuilder`. |

### dir_loader.rs
| Type | What it is |
|------|------------|
| `DirectoryParser` | `new(symbols_path, tasks_path)`, then `load_rules()` / `load_tasks()`. |
| `LoadReport<T>` | `{ value, errors }` from `load_tasks()`: a bad task file lands in `errors`, the good tasks in `value`. |
| `LoadFailure` | `load_rules()`'s error: `Io`, or `Corpus(errors)` — any problem in the `.sym` corpus yields no rule base, and every problem is listed. |
| `LoadError` | `{ message }`, starting with `file:row:col` and carrying the snippet. |

## Grammar

### Top-level shapes

```
task   { goal <term>; [id "…";] [text "…";] [answer <t>, <t>;] <term>; … }
symbol <name> { [<attrs>;] [py "<python source>";] }
rule   { [<attrs>;] <term> (=> | <=>) <term>; [<requirement>, …;] }
```

Rules are top-level blocks in a `.sym` file, not nested inside `symbol`.

### Operator precedence, lowest first

| Rule | Operators | Associativity |
|------|-----------|---------------|
| `deduction` | `=>`, `<=>` | — |
| `or` | `\|\|` | left (`#cache_left_rec`) |
| `and` | `&&` | left |
| `predicate` | `is`, `in`, `==`, `!=`, `<=`, `>=`, `<`, `>` | left |
| `bind` | `as` | — |
| `sum` | `+`, `-` | left |
| `unary` | prefix `-`, `+`, `!` | — |
| `product` | `*`, `/` | left |
| `power` | `^` | right |
| `atom` | literals, identifiers, `(…)`, calls | — |

## Pipeline

```
.sym / .pbl text
  → grammar.rs   Tree<Token>     (byte offsets)
  → lang.rs      Tree<NodeData>  (row/col/len Locations)
  → TermParser / RuleParser / SymbolParser / TaskParser
  → solver types: TermBuf, Rule, SymbolProgram, Task
```

`load_rules()` makes two passes over the `.sym` files: symbols first, so that
every `SymbolProgram` is in the global registry before any rule mentioning it
is parsed. Then every rule, paired with its block's location, is collected
into a `RulesEngine` in one go; a rejection comes back with the rule's input
position, which `load_rules` maps to the file and line.

## Gotchas worth knowing before you edit

- **Embedded Python is a real code path.** A `symbol X { py "…" }` block is
  executed by pyo3 at parse time; the resulting `calculator` serializes the term
  to a JSON dict, hands it to Python wrapped in the `Term` class from
  `py_term.rs`, and reads a `Term` back. `symbols/py/sympy_convert.py` is
  compiled in with `include_str!`, so it does not depend on the working
  directory. No `.sym` in the corpus uses it yet — it works, but it is untested
  by the task corpus.
- **This crate depends on `solver`, and `solver`'s tests depend on this crate.**
  That cycle links two instances of `solver`, so the types are nominally
  distinct across the boundary. See the `SAFETY` notes on `parse_rule` /
  `parse_task` and the serde bridge in `term_with_vars`.
- **A bad task file does not stop `load_tasks`.** Anything that reads
  `LoadReport.value` without looking at `.errors` silently drops broken tasks.
  `load_rules` is the opposite: one bad rule means no base.

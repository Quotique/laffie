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
└── dir_loader.rs   # DirectoryParser — loads a tree of .sym/.pbl, collecting errors
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
| `LoadReport<T>` | `{ value, errors }` — a load **does not** abort on a bad file; the good ones land in `value` and the rest in `errors`. That is how a malformed goal or rule reaches the CLI's `--check` instead of killing the run. |
| `LoadError` | `{ path, message }`, the message already carrying location and snippet. |

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
is parsed. Then rules go to `RulesEngine::register_rule`.

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
- **A bad file does not stop a load.** Anything that reads `LoadReport.value`
  without looking at `.errors` silently drops broken input.

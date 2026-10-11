# Parser Crate — Navigation Map

PEG grammar (the `peg` crate) for `.sym` (symbols + rules) and `.pbl` (tasks).

**No line numbers here, on purpose** — see the note in `solver.md`. Grep the name.

## File Map

```
src/parser/
├── lib.rs          # Location; the entry points term / rule / task / symbol
├── error.rs        # ParserError { loc, msg } + error_string(); LoadError, LoadFailure
├── syntax/         # mod.rs: Span, Expr, Item (syntax::Task/Rule/Symbol); grammar.rs: the PEG `parse`
├── source.rs       # Source — spans back to Locations, slices, span errors
├── scope.rs        # TermScope — `Expr` → TermBuf, resolving names; Names
├── rule.rs         # syntax::Rule::build → Vec<Rule> via RuleBuilder; find-target promotion
├── task.rs         # syntax::Task::build → Task via TaskBuilder
├── symbol.rs       # syntax::Symbol::build → SymbolProgram; PySource::calculator runs embedded Python
├── py_term.rs      # the `Term` class injected into an embedded Python program
└── dir_loader.rs   # DirectoryLoader — loads a tree of .sym/.pbl, collecting errors
```

## Entry points — `lib.rs`

| Function | Parses |
|----------|--------|
| `term(text, names)` | one term; `Names::Params` for rule-like text, `Names::Variables` for task-like |
| `rule(text)` | one `rule` block → `Vec<Rule>` (`zero`/`one` attributes expand it) |
| `task(text)` | one `task` block |
| `symbol(text)` | one `symbol` declaration, unregistered; its `py` block runs |

Each returns `Result<_, ParserError>`, and `ParserError` carries the source
`Location` (row, col) so a message can point at the offending text.

## Key Types

| Type | What it is |
|------|------------|
| `syntax::Expr` | `{ kind, span }` — an expression before names are resolved; an operation's span is its operator's. |
| `syntax::Item` | a top-level `Task` / `Rule` / `Symbol`; attributes stay `Expr` until `build` checks them. |
| `Source` | the parsed text, owned: `loc(span)`, `slice(span)`, `error(span, msg)`, `uint(span)`, `ident(expr)`. |
| `TermScope` | resolves names against the symbol registry. One scope spans a whole rule (pattern and requirements share `as` params and `..` ids) or one task term; task ids depend on it. |
| `DirectoryLoader` | `new(symbols_path, tasks_path)`, then `load_rules()` / `load_tasks()`. |
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

### Operator precedence, lowest first (`syntax/grammar.rs`)

| Rule | Operators | Associativity |
|------|-----------|---------------|
| `term` | `=>`, `<=>` | does not chain |
| `logic` | `\|\|`, then `&&`, then prefix `!` | left |
| `compare` | `is`, `in`, `==`, `!=`, `<=`, `>=`, `<`, `>` | does not chain |
| `bind` | `as` | — |
| `arith` | `+`, `-` | left |
| `signed_product` | a leading `-`/`+` takes the whole product: `-a/b` is `-(a/b)` | — |
| `product` | `*`, `/` | left |
| `factor` | prefix `-`: `a * -b` | — |
| `juxt` | `2x`, `2(x+1)`, `(a)(b)`: a number or a parenthesis on the left | left |
| `power` | `^`, signed exponent | right |
| `primary` | `(…)`, `find(…) {…}`, calls, `..`, numbers, names, strings | — |

## Pipeline

```
.sym / .pbl text
  → syntax::grammar::parse::file   Vec<Item>   (byte spans)
  → {Task,Rule,Symbol}::build(&Source)   TermScope resolves names
  → solver types: TermBuf, Rule, SymbolProgram, Task
```

`load_rules()` makes two passes over the `.sym` files: symbols first, so that
every `SymbolProgram` is in the global registry before any rule mentioning it
is built. Then every rule, paired with its item's span, is collected into a
`RulesEngine` in one go; a rejection comes back with the rule's input
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

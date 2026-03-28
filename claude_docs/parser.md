# Parser Crate — Navigation Map

PEG-based grammar for `.sym` (symbol/rule) and `.pbl` (task) files.

## File Map

```
src/parser/
├── lib.rs          # Root: Location, NodeData, Tree/Node type aliases, CompactString
├── error.rs        # ParserError with source location display
├── grammar.rs      # PEG grammar (peg crate) — tokenization and expression rules
├── lang.rs         # Public API: wraps grammar with Location annotation
├── term.rs         # TermParser — AST → TermBuf conversion
├── rule.rs         # RuleParser — AST → Vec<Rule> via RuleBuilder
├── symbol.rs       # SymbolParser — AST → SymbolProgram
├── task.rs         # TaskParser — AST → Task via TaskBuilder
└── dir_loader.rs   # DirectoryParser — loads .sym/.pbl from filesystem
```

## Key Types

### lib.rs
| Line | Type | Description |
|------|------|-------------|
| 23 | `Location` | `{ col, row, len }` — source position |
| 30 | `NodeData` | `{ symbol: CompactString, location: Location }` — annotated AST node |
| 35 | `Tree` | `trees::Tree<NodeData>` |
| 36 | `Node` | `trees::Node<NodeData>` |

### error.rs
| Line | Type | Description |
|------|------|-------------|
| 10 | `ParserError` | `{ loc: Location, msg: String }` — with `error_string()` for formatted display |

### lang.rs — Public API
| Line | Function | Description |
|------|----------|-------------|
| 5 | `symbol(text) -> Result<Tree>` | Parse symbol declaration |
| 15 | `terms(text) -> Result<Vec<Tree>>` | Parse semicolon-separated terms |
| 30 | `lang_rule(text) -> Result<Tree>` | Parse rule block |
| 40 | `task(text) -> Result<Tree>` | Parse task block |
| 50 | `any(text) -> Result<Vec<Tree>>` | Parse mixed content (tasks, rules, symbols) |

### term.rs
| Line | Type | Description |
|------|------|-------------|
| 11 | `TermParser` | `{ params, with_var, last_arglist_id }` |
| 18 | `with_variables()` | Enable variable parsing (for tasks) |
| 23 | `with_params()` | Pre-set parameter substitutions |
| 28 | `try_parse()` | Entry: `&Node → Result<TermBuf>` |

### rule.rs
| Line | Type | Description |
|------|------|-------------|
| 13 | `RuleParser<'a>` | `{ syntax_tree, func_symbol }` |
| 31 | `parse()` | Entry: parses attributes, term (`=>` / `<=>`), requirements → `Vec<Rule>` |
| 82 | `parse_attribute()` | Handles: level, goal, zero, one, id, block, normalize, subtree, equivalence, replace |

### symbol.rs
| Line | Type | Description |
|------|------|-------------|
| 7 | `SymbolParser<'a>` | `{ ast }` |
| 16 | `parse()` | Entry: extracts name + attributes → `SymbolProgram` |
| 46 | `parse_attr()` | Handles: infix(u64), display(str), associative, commutative |

### task.rs
| Line | Type | Description |
|------|------|-------------|
| 15 | `TaskParser<'a>` | `{ syntax_tree }` |
| 24 | `parse()` | Entry: extracts goal, text, answers, conditions → `Task` |

### dir_loader.rs
| Line | Type | Description |
|------|------|-------------|
| 17 | `DirectoryParser` | `{ symbols_path, tasks_path }` |
| 30 | `load_rules()` | Loads all `.sym` files → `RulesEngine` |
| 59 | `load_tasks()` | Loads all `.pbl` files → `Vec<Task>` |
| 81 | `load_symbols()` | First pass: registers SymbolPrograms before rules |

## PEG Grammar — Operator Precedence (grammar.rs)

From lowest to highest:

| Priority | Rule (line) | Operators | Associativity |
|----------|-------------|-----------|---------------|
| 1 | `deduction` (158) | `=>`, `<=>` | — |
| 2 | `or` (162) | `\|\|` | left (`#cache_left_rec`) |
| 3 | `and` (167) | `&&` | left (`#cache_left_rec`) |
| 4 | `predicate` (172) | `is`, `in`, `==`, `!=`, `<=`, `>=`, `<`, `>` | left (`#cache_left_rec`) |
| 5 | `bind` (176) | `as` | — |
| 6 | `sum` (181) | `+`, `-` | left (`#cache_left_rec`) |
| 7 | `unary` (191) | prefix `-`, `+`, `!` | — |
| 8 | `product` (201) | `*`, `/` | left (`#cache_left_rec`) |
| 9 | `power` (206) | `^` | right |
| 10 | `atom` (210) | literals, idents, `(...)`, fn calls | — |

## Top-level Grammar Constructs

```
task   = "task" "{" goal_decl text_decl? answer_decl* term* "}"
symbol = "symbol" name "{" attr* "}" rule*
rule   = "rule" "{" attr_block? term ";" requirement* "}"
```

## Parsing Pipeline

```
Source text (.sym / .pbl)
  → grammar.rs (PEG tokenizer) → Tree<Token> with byte offsets
  → lang.rs (annotation) → Tree<NodeData> with Location(row, col, len)
  → TermParser / RuleParser / SymbolParser / TaskParser
  → Solver domain types (TermBuf, Rule, SymbolProgram, Task)
```

## DirectoryParser Flow

```
DirectoryParser::new(symbols_path, tasks_path)

load_rules():
  1. load_symbols() — first pass, registers SymbolPrograms globally
  2. Iterate .sym files again:
     - Parse symbol blocks → SymbolParser → register
     - Parse rule blocks → RuleParser → RulesEngine.register_rule()
  → RulesEngine

load_tasks():
  1. Iterate .pbl files:
     - Parse task blocks → TaskParser → normalize terms
  → Vec<Task>
```

# View, Database, Utils Crates — Navigation Map

## View Crate

Solution rendering with pluggable backends.

### File Map

```
src/view/
├── lib.rs       # Renderer trait, View struct
├── console.rs   # Console impl — colored terminal output
├── html.rs      # Html impl — HTML-escaped output
└── tui.rs       # Tui impl — ratatui Line/Span output
```

### Key Types

#### lib.rs
| Line | Type | Description |
|------|------|-------------|
| 16 | `Renderer` trait | `display_goal()`, `display_term()`, `display_answer()`, `dump_frame()` |
| 33 | `View<'a>` | `{ solution: &Solution, rendered: HashSet<TermBuf> }` |
| — | `View::display_impl()` | Iterates solution steps via `StepsSource`, calls renderer methods |

#### Implementations
| File | Struct | Output | Key Dep |
|------|--------|--------|---------|
| console.rs:18 | `Console` | Colored text (`fmt::Write`) | `colored` |
| html.rs:16 | `Html` | HTML string (`fmt::Write`) | `html_escape` |
| tui.rs:18 | `Tui` | `Vec<Line<'a>>` for ratatui | `ratatui` |

---

## Database Crate

Embedded KV persistence. **Currently being rewritten** from `sled` + bincode
onto `redb` + JSON/zstd; the schema is in transition.

### File Map (current)

```
src/database/
├── lib.rs    # Re-exports TaskDb, TaskRecord
└── task.rs   # TaskRecord, TaskDb — minimal task storage
```

### Key Types

#### task.rs
| Type | Description |
|------|-------------|
| `TaskRecord` | Fields: id(u128), text, group, givens, goal, answer, runs(Vec<usize>), reports(Vec<u64>) |
| `TaskDb`     | `open()`, `get()`, `put()`, `remove()`, `iter()` |

The new schema (planned): two redb tables — `tasks: TaskId([u8;16]) -> Task`,
`runs: (TaskId, u64) -> Run` — with FIFO eviction at 10 runs per task.
`TaskId` is `blake3(canonical(sorted_givens, goal))[..16]`. Runs carry a
structured `SolutionTrace` mirror of `solver::task::Solution` (no `SharedRule`
or `SharedSolution`; rules referenced via `RuleRef::Named|Anonymous`).

---

## Utils Crate

Logging infrastructure and helper utilities.

### File Map

```
src/utils/
├── lib.rs              # Re-exports
├── subset.rs           # SubsetIterator — combinatorial distributions
├── trees_index.rs      # TreeIndex, IndexedTree trait
└── logger/
    ├── mod.rs          # Re-exports
    ├── config.rs       # Config struct, init() → slog logger
    ├── filter.rs       # Filter — per-module log level filtering
    └── format.rs       # Custom log header formatting
```

### Key Types

#### subset.rs
| Line | Type | Description |
|------|------|-------------|
| 14 | `SubsetIterator` | Distributes N elements into K subsets. `new(container_len, subset_count)` |

#### trees_index.rs
| Line | Type | Description |
|------|------|-------------|
| 8 | `TreeIndex(Vec<usize>)` | Path to a node in a generic tree |
| 10 | `IndexedTree` trait | `get(&TreeIndex)`, `get_mut(&TreeIndex)`, `id()` |
| 18 | `impl for Node<T>` | Navigate tree by index path |
| 55 | `impl for Tree<T>` | Same, starting from root |

#### logger/config.rs
| Line | Type | Description |
|------|------|-------------|
| 16 | `Config` | Fields: channel_size, filename, level, num_files, file_rotate_bytes, target_levels |
| — | `Config::init()` | Sets up slog-async + file-rotate + per-module filtering → `GlobalLoggerGuard` |

#### logger/filter.rs
| Line | Type | Description |
|------|------|-------------|
| 7 | `Filter` | Prefix-based module filtering using patricia_tree. Caches results. |

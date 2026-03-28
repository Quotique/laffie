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

Embedded KV persistence via `sled`.

### File Map

```
src/database/
├── lib.rs    # Re-exports TaskDb, UserDb, records
├── task.rs   # TaskRecord, TaskDb — task storage with versioning
└── user.rs   # UserRecord, UserDb — user storage with task history
```

### Key Types

#### task.rs
| Line | Type | Description |
|------|------|-------------|
| 37 | `TaskRecord` | Fields: version, id(u128), text, group, givens, goal, answer, runs(Vec<usize>), reports(Vec<u64>) |
| 52 | `TaskDb` | `open()`, `get()`, `get_or_insert()`, `put()`, `remove()`, `iter()`, `backup()`, `restore()` |
| 106 | `is_same()` | Compares goals and givens (ignores answers/runs) |
| 216 | `compose_id()` | `(number: u64, task_id: u64) -> u128` — upper 64 = run number, lower 64 = task_id |
| 225 | `split_id()` | Reverse of compose_id |

#### user.rs
| Line | Type | Description |
|------|------|-------------|
| 25 | `UserRecord` | Fields: version, id(u64), locale(String), tasks(BTreeSet<u128>) |
| 34 | `UserDb` | `open()`, `get()`, `put()`, `backup()`, `restore()` |
| 50 | `add_task_id()` | Adds task to user's history |

Both `TaskDb` and `UserDb` support version migration via `iter_old()`.

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

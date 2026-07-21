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

Single-file persistence on `redb`. Values are zstd-compressed JSON, so
schema evolution rides on `Option`/`#[serde(default)]` rather than
per-record version tags.

Two tables in one file:

| Table | Key | Value |
|-------|-----|-------|
| `tasks` | `TaskId` (`[u8;16]`) | encoded [`Task`](#task) |
| `runs`  | `TaskId ‖ seq` (`[u8;24]`, BE seq) | encoded [`Run`](#run) |
| `meta`  | `&str` (`"schema_version"`) | `u64` schema version |

`Db::open` checks `meta` against `SCHEMA_VERSION` and refuses a mismatched or
pre-versioning (tasks but no marker) file with a clear error.

### File Map

```
src/database/
├── lib.rs    # Module wiring + re-exports
├── id.rs     # TaskId, compute_task_id (blake3 over Display text), id_to_hex / id_from_hex
├── task.rs   # Task DTO, From<&solver::Task>, From<Task> for solver::Task
├── run.rs    # Run, RunStats, Run::from_solution
├── trace.rs  # SolutionTrace (mirror), TraceTerm, TraceInference, RuleRef, TraceParams
├── codec.rs  # zstd + serde_json encode/decode
└── db.rs     # Db struct, redb tables, transaction-bounded API
```

### Identifiers

`TaskId = [u8; 16]`. Computed by `id::compute_task_id`:

```
blake3(b"laffie:task:v2"
       || (len(g) || g  for g in sort(text(givens)))
       || b"|goal|"
       || text(goal))[..16]
```

`text(t)` is `t.to_string()` (canonical Display), not serde bytes, so the id is
stable across serialization refactors. Equivalent tasks (same givens-multiset
and goal) collapse to the same id; `possible_answers` is intentionally outside
the hash. Convert to/from hex with `id_to_hex` / `id_from_hex`.

### Task

`task.rs` — `Task { id, name, text, group, givens, goal, possible_answers, hidden, created_at }`.
`From<&solver::task::Task>` computes the id and carries `name`; the inverse
(`From<Task> for solver::Task`) restores `name` and sets `solver::Task::id` via
`solver::task::content_id` (location-independent hash of the terms), not by
truncating the 128-bit `TaskId`.

### Run

`run.rs` — `Run { task_id, seq, created_at, stats, solution }`.
`stats: RunStats { cycles, status, answer, duration_ms }` where `answer`
duplicates the corresponding `solution.terms[idx]` for cheap lookups and
`duration_ms` is reserved for when wall-clock timing gets wired in.
`Run::from_solution(task_id, &Solution)` builds everything from a finished
solver run; `seq` is overwritten by [`Db::add_run`].

### SolutionTrace

`trace.rs` mirrors `solver::task::Solution` for persistence. Runtime-only
state (caches, `Arc` graphs of `SharedRule`/`SharedSolution`) is stripped;
sub-solutions and rule references become indices into flat `Vec`s.

```
SolutionTrace
├── status         : TraceStatus (NotDone | Answer(idx) | Err(String))
├── terms          : Vec<TraceTerm>
├── sub_solutions  : Vec<SolutionTrace>     // pool for recursive references
└── find_bindings  : Vec<(TermBuf, idx)>    // multi-var find(x, y, …)

TraceTerm { term, inference: TraceInference }

TraceInference
├── Condition
├── Rule { parent, rule_ref: RuleRef, params: TraceParams,
│          requirements: Vec<idx into sub_solutions> }
└── Transform { parent, sub_solution: idx into sub_solutions }

RuleRef
├── Named(String)        // .sym `id` attribute
└── Anonymous([u8; 8])   // blake3(json(rule.term))[..8]

TraceParams { params: Vec<(String, TermBuf)>,
              arglists: Vec<(u64, Vec<TermBuf>)> }
```

`From<&Solution> for SolutionTrace` walks `solution.terms` once,
allocating sub-trace indices on first sight of each `requirements`/
`Transform.solution`.

### Db API (`db.rs`)

`Db::open(path)` opens (and initializes) the redb file at `path`.

| Method | Purpose |
|--------|---------|
| `put_task(&Task)` | Idempotent upsert keyed by `task.id` |
| `get_task(TaskId)` / `iter_tasks()` / `task_count()` | Read |
| `remove_task(TaskId)` | Cascades to `runs` of that task in one tx |
| `set_hidden(TaskId, bool)` | Toggles the `hidden` flag |
| `add_run(Run) -> Run` | Assigns next per-task `seq`, evicts down to `RUNS_PER_TASK_LIMIT` (10) |
| `runs_of(TaskId)` / `last_run(TaskId)` | Newest-first range scan over `(id, *)` |

Every method runs in its own redb transaction; writes commit before
returning.

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

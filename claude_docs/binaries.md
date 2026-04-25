# Binary Applications — Navigation Map

Active workspace binaries: `cli`, `tui`. `tgbot` is archived (see bottom of this file).

## CLI (`src/bin/cli/`)

Batch solver: loads symbols/tasks from disk, solves all, reports statistics.

### Files
- `main.rs` (220 lines) — argument parsing, main loop, statistics
- `settings.rs` (23 lines) — YAML config deserialization

### CLI Arguments (main.rs:20-54)
| Flag | Default | Description |
|------|---------|-------------|
| `--config` | `./config/local.json` | Config file path |
| `--only` | `""` | Filter tasks by ID prefix/suffix |
| `--remove` | — | Remove task from DB by hex ID |
| `--symbols` | — | Override symbols directory |
| `--tasks` | — | Override tasks directory |
| `--tasks_db` | `./db/tasks` | Tasks DB path |
| `--trace` | false | Enable solution trace dumping |
| `--exec_deadline` | 100000 | Max execution cycles |

### Main Flow (main.rs:78-219)
1. Parse args → load settings → init logger
2. `DirectoryParser::new()` → `load_rules()`, `load_tasks()`
3. For each task (filtered by `--only`):
   - `Solver::new(rules)` → `solve(task, tracer, deadline)`
   - `View::display_impl(&Console)` — render to stdout
   - Validate answer, track stats per group
4. Print per-group and total statistics

---

## TUI (`src/bin/tui/`)

Interactive terminal UI with multi-tab navigation and async solving.

### Files
```
src/bin/tui/
├── main.rs       # Event loop, terminal setup, key handling
├── state.rs      # TuiState — rules, tasks tree, solve queue
├── pane.rs       # Pane — widget layout composition
├── ui.rs         # Ui — tab management, solver worker spawning
├── settings.rs   # YAML config with theme
├── theme.rs      # Color/style constants for ratatui
└── widgets/
    ├── mod.rs
    ├── popup.rs              # Centered popup overlay
    ├── rules_list.rs         # F1 left: scrollable rule list
    ├── rule_window.rs        # F1 right: selected rule details
    ├── tasks_list.rs         # F2 left: tree-view of tasks by directory
    ├── solution_window.rs    # F2 right: solution steps display
    ├── tracing_navigation.rs # F3 left: solution term tree with requirements
    └── tracing_window.rs     # F3 right: term/rule detail view
```

### Tabs (ui.rs:58-65)
| Key | Tab | Left Pane (40%) | Right Pane (60%) |
|-----|-----|-----------------|------------------|
| F1 | Rules | RulesList | RuleWindow |
| F2 | Tasks | TasksList (tree) | SolutionWindow |
| F3 | Tracing | TracingNavigation | TracingWindow |

### Key Controls (main.rs:30-93)
- Arrow keys / hjkl / Russian equivalents — navigation
- Enter — solve selected task
- `a` — solve all tasks
- `r` — reload rules from disk
- `c` — cancel running solver
- Space — toggle tree node expand/collapse

### State (state.rs)
| Line | Type | Description |
|------|------|-------------|
| 19 | `TuiState` | `rules_engine`, `tasks` (Tree), `solve_queue`, `settings` |
| 31 | `TasksNode` | Enum: `Task(TaskState)`, `Directory(DirectoryStat)` |
| 37 | `TaskState` | `solution`, `solution_pos`, `tracing_state` (breadcrumb) |
| 44 | `DirectoryStat` | Counts: solved, unsolved, wrong_answer, not_started |

### Solver Worker (ui.rs:104-145)
- Spawns `std::thread` processing `solve_queue`
- Progress via `Arc<Mutex<SolverProgress>>` with `ProgressReporter` implementing `Tracer`
- Returns `Vec<(TreeIndex, SharedSolution)>`

---

## Telegram Bot (`src/bin/tgbot/`) — **archived**

Excluded from `[workspace.members]`; not built by `cargo build --workspace`.
Reviving requires a `telegram-bot` crate upgrade and a rewrite onto the new
`Db` API (and a separate decision about user-data storage, since `UserDb` was
removed). Source remains in-tree for reference.

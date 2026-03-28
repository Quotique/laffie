# Binary Applications — Navigation Map

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

## Telegram Bot (`src/bin/tgbot/`)

Async Telegram interface for task submission and solving.

### Files
```
src/bin/tgbot/
├── main.rs             # Tokio async entry, bot stream processing
├── settings.rs         # Config: api_token, db paths, symbols_dir
├── text.rs             # Message templates: system(), version(), task_text()
├── pagination.rs       # Paginator — splits long messages into pages
└── commands/
    ├── mod.rs          # Command routing, start_handler, static handlers
    ├── task.rs         # Inline task parsing → solve → HTML response
    ├── tasks_list.rs   # User's task history with Rerun/Report buttons
    ├── rerun.rs        # Re-solve existing task by ID
    └── report.rs       # Report issue with a task
```

### Commands
| Command | Handler | Description |
|---------|---------|-------------|
| `/start` | `start_handler` (mod.rs:59) | Welcome message with buttons |
| `/help` | static (mod.rs:124) | Help text (i18n) |
| `/guide` | static | Usage guide |
| `/examples` | static | Example problems |
| `/tasks` | `tasks_list_handler` | User's solved tasks |
| (text) | `task_handler` | Parse and solve inline task |
| (callback) `rerun:ID` | `rerun_handler` | Re-solve by task ID |
| (callback) `report:ID` | `report_handler` | Report task issue |

### Key Details
- Uses `rust_i18n` with Russian locale
- `Paginator` splits responses at 4096-byte Telegram limit
- Renders via `Html` renderer
- Stores results in `TaskDb` + `UserDb`

---

## Database Migration (`src/bin/migrate_db/`)

Upgrades database format between versions.

### Files
- `main.rs` (65 lines) — migration logic with backup/restore
- `settings.rs` (25 lines) — paths for DBs and backups

### Flow (main.rs:38-64)
1. Backup existing DB
2. Iterate `iter_old()` → convert to current record format → `put()`
3. On error: `restore()` from backup

# Binary Applications — Navigation Map

Workspace binaries: `cli`, `tui`. `tgbot` is archived (bottom of this file).

**No line numbers here, on purpose** — see the note in `solver.md`. Grep the name.

## CLI (`src/bin/cli/`)

Batch solver: loads symbols and tasks from disk, solves, persists to the
redb-backed `database::Db`, reports statistics.

### Files
- `main.rs` — `Args` (clap derive), the run loop, statistics
- `settings.rs` — YAML config
- `check.rs` — `--check` linter: parse errors, dangling block refs, suspicious params, term roundtrip
- `run_diff.rs` — `RunDiff`: this run against the task's last stored run
- `explain.rs` — `ExplainTracer` / `ExplainReport` for `--explain`
- `coverage.rs` — `CoverageTracer` / `CoverageReport` for `--coverage`

### Arguments
| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--config` | `-c` | `config/cli.yaml` | config file |
| `--only` | `-o` | `""` | task names or 32-hex `TaskId` prefixes, comma-separated |
| `--remove` | `-r` | `""` | remove a task from the DB by id (cascades to its runs) |
| `--symbols` | `-s` | — | override the symbols directory |
| `--tasks` | `-p` | — | override the tasks directory — this is how a task set is picked |
| `--db-path` | `-d` | `./db/tasks.redb` | redb file |
| `--trace` | `-t` | false | dump the solver trace to a file |
| `--exec-deadline` | `-e` | 100000 | cycle budget per task |
| `--time-limit` | `-l` | 86400 | wall-clock seconds per task |
| `--record` | — | false | persist each run as the new baseline; off means read-only, the diff is still printed |
| `--check` | — | false | lint instead of solving; non-zero exit on errors |
| `--strict` | — | false | with `--check`, warnings become errors |
| `--explain` | — | false | per-task cycle breakdown; best with `--only` |
| `--coverage` | — | false | after the run, list dead rules, split into never-fired and fired-but-useless |

### Flow
1. Args → settings → logger.
2. `DirectoryParser::new()` → `load_rules()`, `load_tasks()`. **Both return a
   `LoadReport`**, and `main` prints `.errors` before solving — a broken file
   does not abort the run.
3. `Db::open(db_path)`, creating the parent directory if missing.
4. Per task (filtered by `--only`): store the task, `Solver::solve`, render
   through `View` to the console, validate against `answer` if the task
   declares one, accumulate group statistics, and record the run when
   `--record` is set (FIFO past `RUNS_PER_TASK_LIMIT`).
5. Per-group and total statistics.

## TUI (`src/bin/tui/`)

Interactive terminal UI over the same engine, solving on a worker thread.

### Files
```
src/bin/tui/
├── main.rs       # terminal setup, event loop, key/mouse → Command
├── ui.rs         # Ui, Tab, Command; tab layout and the solver worker
├── state.rs      # State — rules, task tree, per-task solutions
├── pane.rs       # Pane — widget layout composition
├── settings.rs   # YAML config, including the theme name
├── theme.rs      # named colour themes for ratatui
├── strings.rs    # all user-facing text, in one place
└── widgets/
    ├── popup.rs               # centred overlay
    ├── rules_list.rs          # Rules tab, left
    ├── rule_window.rs         # Rules tab, right
    ├── tasks_list.rs          # Tasks tab, left: directory tree
    ├── solution_window.rs     # Tasks tab, right: solution, status, run diff
    ├── tracing_navigation.rs  # Tracing tab, left: term tree with requirements
    ├── tracing_window.rs      # Tracing tab, right: term and rule detail
    ├── settings_view.rs       # Settings tab
    └── solver_progress.rs     # progress while the worker runs
```

### Tabs — `Tab` in `ui.rs`
| Key | Tab | Left | Right |
|-----|-----|------|-------|
| F1 | Rules | `RulesList` | `RuleWindow` |
| F2 | Tasks | `TasksList` | `SolutionWindow` |
| F3 | Tracing | `TracingNavigation` | `TracingWindow` |
| F4 | Settings | `SettingsView` | — |

### Keys — `Command` in `ui.rs`, mapped in `main.rs`
Every letter has its Russian-layout twin, so `s`/`ы` are the same key.

| Key | Command |
|-----|---------|
| arrows, `hjkl` | `Left`/`Down`/`Up`/`Right` |
| Enter, Space | `Toggle` — expand or collapse a tree node |
| `s` | `Solve` the selected task |
| `a` | `SolveAll` |
| `r` / `R` | `Reload` / `ReloadAll` from disk |
| `e` | `EditSelected` — open the source in `$EDITOR` |
| `c` | `Cancel` the running solve |
| `/` then text | filter; `?` shows help |
| ctrl-u / ctrl-d | page up / down |
| ctrl-s | save settings |
| `q` | quit |

### State — `state.rs`
| Type | What it is |
|------|------------|
| `State` | rules engine, task tree, settings, solve queue. |
| `TasksNode` | tree node: `Task(TaskState)` or `Directory(DirectoryStat)`. |
| `TaskState` | `solution`, `previous_solution` (for the run diff), `solution_pos` (scroll), `tracing_state` — the breadcrumb of solutions the Tracing tab has descended into. |
| `DirectoryStat` | counts per directory: solved, unsolved, wrong answer, not started. |
| `ProblemTask` / `TaskStatusKind` | the flat list of problem tasks a directory summary lists, and the badge each gets. |

The worker runs on a `std::thread` over the solve queue, reporting through a
`Tracer` implementation so the UI can show progress and cancel through the
`CancelToken`.

## Telegram Bot (`archive/tgbot/`) — archived

Excluded from `[workspace.members]`; not built by `cargo build --workspace`.
Reviving it needs a `telegram-bot` upgrade and a rewrite onto the current `Db`
API, plus a decision about user-data storage since `UserDb` was removed.

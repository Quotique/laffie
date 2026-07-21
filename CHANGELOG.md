# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Search-loop panics are now recoverable: a term-stack overflow propagates as
  `SolveError::StackOverflow`, subtask build failures surface as an errored
  subtask solution (`SolveError::Internal`), and a subtask condition whose
  parent was not copied over degrades to a parent-less term
- A rule whose `block(...)` reference never resolves is reported as a load
  error (listing the undefined ids) instead of panicking `suggest_rules` at
  solve time

### Changed
- cli: exit with a non-zero code on failure — a rule/task load error, a
  broken config/paths/db, or any wrong answer in the corpus. Unsolved tasks
  are not treated as failures. A wrong answer is now counted only under
  `wrong answer`, no longer double-counted as `solved`
- Directory loading no longer swallows errors or aborts on the first broken
  file: `load_rules` / `load_tasks` return a `LoadReport` collecting every
  parse error (with path and location), and the loader keeps going. cli lists
  all load errors and refuses to run a broken corpus; the TUI continues with
  whatever loaded
- Block type is dispatched by its root token (`Declare` / `Rule` / `Task`)
  instead of by which parser happened to fail, so a broken symbol is reported
  as a symbol error. Files are loaded in a deterministic (sorted) order, and a
  symbol's Python program runs once instead of twice

## [0.6.0] - 2026-07-09

### Added
- TUI: solve tasks in parallel with rayon; the solve queue stays alive
  while a worker runs, and input blocks when idle
- TUI: Settings tab (F4) showing runtime values, editable and persisted
  back to yaml with Ctrl+S
- TUI: diff of a solution against the previous run, with a rule-aware
  multiset diff in the solution window
- TUI: mouse support (wheel scroll and click) with precise tab hit-boxes;
  expanded keymap with paging, panels, a help bar, and esc handling
- TUI: incremental filter for the rules list; edit the selected source in
  `$EDITOR`; Shift+R reloads tasks alongside rules; per-directory task
  listing in the summary panel
- TUI: theme as data with dark / light / high-contrast presets
- Atomic piecewise-answer recognition in the solver (`check_find_answer` /
  `is_answer_form`): a single-target `find(x)` answer is accepted directly
  when it is a leaf (`x == known` / `x in known`) or a `||` of `&&`-branches
  (one leaf + `is known` guards). Replaces the rule-based leaf-wrap assembly
  and removes its combinatorial blow-up on multi-branch answers
- Kleene truth checkers for `&&` / `||` so `!(X && Y)`-style requirements
  evaluate to a definite truth
- `parents()` context primitive for rule requirements: resolves to the
  `set(...)` of head symbols of the match position's ancestors, enabling
  guards like `!(answer in parents())`; replaces the `block(...)`/
  complement-pair machinery for piecewise-answer detection. Modeled like
  `solve`: an inert symbol (parsing only) computed by a dedicated
  requirement pass (`resolve_parents_in_hypothesis`) against the match
  position carried on the hypothesis — not by term normalization
- Per-problem wall-clock time limit (`cli --time-limit`, seconds)
- Hypothesis grounding: `Hypothesis` / `GroundedHypothesis` split, free
  params bound via `==` requirements and cartesian over `set(...)`
  generators, with `match_term!` and a `Substitute` trait as supporting
  infrastructure
- Multi-variable `find(x, y, ...)` with conjunctive answer assembly and
  commutative goal matching
- Equations with rational roots can now be solved
- `database` rewritten on `redb` + zstd-JSON: content-addressed
  `TaskId`, per-task `Run` history (FIFO-capped at 10), structured
  `SolutionTrace` mirror of `Solution`

### Changed
- `move_left` no longer canonicalizes an already-solved `x == known` (guard
  `!(a is atom && b is known)`), so it is recognized as the answer
- Equation-solving rules emit raw piecewise resolutions instead of wrapping
  them in `answer(...)`; recognition is now atomic in the solver
- `cli` persists every solve through the new `database::Db`; DB-path
  flag renamed `--tasks-db` → `--db-path`, hex `TaskId` for `--remove` /
  `--only`; `--only` accepts a comma-separated list of task ids / hex
  prefixes
- Term codec returns structured `DecodeError`s instead of panicking

### Removed
- Rule-based answer assembly: the `L1/L2/L3` leaf-wrap/merge rules in
  `answer.sym` and the pure leaf-wraps (`x in set/Reals/empty_set => answer`,
  `x == a || x == b => answer(x in set)`); superseded by atomic recognition
- `tgbot` excluded from the workspace (sources kept in-tree as archived);
  `migrate_db`, `UserDb`, and the sled/bincode-era `database` shim
  removed along with their dependencies

### Fixed
- TUI: removed panic sources in the render path; SolveAll on a directory
  with sub-directories no longer panics
- More equations needing bracket/power expansion now solve
- Equations with no real solutions now return the empty set instead of an
  unfinished answer
- A CAS error while solving no longer crashes the run
- Saving a solution no longer runs out of memory on problems with
  heavily reused sub-results
- Solving is now deterministic — a task no longer succeeds on some runs
  and fails on others
- Workspace-wide clippy cleanup
- `from_sympy` losing the param/variable distinction
- Hardcoded Telegram API token replaced with a placeholder

### Docs
- `code_structure_ru.md` refreshed for the redb schema and the current
  solver surface

## [0.5.0] - 2025-12-20

### Added
- TUI application: task tree, progress bar, task cancellation, tracing navigation,
  profiler, rules reloading, subtree execution, Russian layout support
- Answer validation in solver
- Hypothesis tracing
- Dynamic max level selection for solver
- New rules for irrational equations
- Minus symbol replaced with -1 multiplication
- Rule deduplication per iteration
- LICENSE file

### Changed
- Major solver refactoring: new inference structure, term encapsulates tree interface,
  symbols merged into term module, TermProps as bitflags, `mcore` renamed to `solver`
- TUI refactoring: theme system, widget extraction, state management
- "Purpose" renamed to "goal", proof/prove/proven terminology unified

### Fixed
- Incorrect proof for "is known" property
- Max level bug in solver
- Logger memory consumption
- Panics on TUI startup
- Incorrect plus normalization
- Wrong cache state on subtasks limit exceed
- Minus symbol mapping and parsing

## [0.4.2] - 2024-08-23

### Changed
- Workspace crate versions unified

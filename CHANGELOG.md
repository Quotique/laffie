# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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
- `cli` persists every solve through the new `database::Db`; DB-path
  flag renamed `--tasks-db` → `--db-path`, hex `TaskId` for `--remove` /
  `--only`; `--only` accepts a comma-separated list of task ids / hex
  prefixes
- Term codec returns structured `DecodeError`s instead of panicking

### Removed
- `tgbot` excluded from the workspace (sources kept in-tree as archived);
  `migrate_db`, `UserDb`, and the sled/bincode-era `database` shim
  removed along with their dependencies

### Fixed
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

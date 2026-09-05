# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- A goal that is not `find`/`prove`/`transform` is a load error naming the file,
  the line and the goal, instead of a panic partway through the run.
- A store overflow while assembling a multi-target answer ends the task with an
  error instead of going unnoticed.
- A subtask's trace no longer repeats the steps its parent took to reach the
  conditions it inherited.
- API: `Task::goal()` replaces the field, a `Task` is built only through
  `TaskBuilder`, a stored task converts with `TryFrom`, `Solver::solve` takes
  `&self`, and `Solution`, `TermProps`, `Solver`, `Tracer` and the step walker
  live in `solver::engine` rather than `solver::task`.
- A multi-variable `find` answer shows how each unknown was derived. The
  answer term is no longer fabricated into the solution's store, so the walk
  back to the conditions starts from every part.
- A subtask already shown in the output leaves a reference instead of nothing,
  so a `needed: [...]` line always points at a derivation that is there.
- API: `SolutionStatus::Answer(idx)` becomes `Answered(Arc<task::Answer>)`,
  with one part per unknown; `Solution::find_bindings` and `validate_answer`
  are gone, the latter replaced by `Answer::matches`. The status is no longer
  `Copy`, and `Renderer::display_answer` takes the answer rather than a term.
- API: `RunControl` is now `Limits`; `Limits::init` is unchanged.
- The run trace schema is version 3. Existing databases are refused with an
  error naming the mismatch; back them up and recreate.

## [0.7.0] - 2026-07-21

### Added
- Inequality proving over `>`, `>=`, `<`, `<=`: sign/bound/power/estimate lemmas
  and a `bound_implies` check (`x^2 >= 0`, `x*y > 6`, …; `doc/ru/inequalities.md`).
- `<expr> is even` predicate → unconditional even-power lemmas (`x^4 >= 0`,
  `x^2 + y^2 + 1 > 0`).
- cli `--coverage`: lists dead rules (never-fired vs fired-but-useless).
- cli `--explain`: per-task cycle / focus / rule / subtask breakdown.
- cli `--check`: domain linter — load errors and near-miss symbol names
  (`--strict` promotes warnings; non-zero exit on errors).
- cli regression diff vs the task's last run (`NEWLY FAILING` / `SOLVED`,
  `ANSWER CHANGED`, `SLOWER >2x`); recording opt-in via `--record`.

### Changed
- Numbers are exact rationals (`num::BigRational`); some answers change form
  (values unchanged) and `13^(1/2)` no longer truncates.
- `TaskId` / task id are content hashes stable across reformat and serde; redb
  carries a `schema_version` and stores the task `name`. Recreate databases.
- Cancellation via a first-class `RunControl` (budget + deadline + `CancelToken`);
  `Tracer` is now purely observational.
- Commutative / AC matching is a budgeted lazy backtracking search (no more
  permutation/partition blow-up or completeness cliff).
- Search hot-path ~11% faster (`slow` corpus, answers unchanged): lazy grounding,
  min-heap agenda, `Arc` substitutions, incremental subterm paths.
- A `prove` goal reducing to a trivial truth (e.g. `x^2 >= 0`) needs no witness
  term.
- cli: single `config/cli.yaml`, pick tasks with `-p/--tasks`; `--config`
  default fixed.
- cli: non-zero exit on failure (load error / broken config / wrong answer);
  wrong answers no longer counted as solved.
- Directory loading collects all errors (`LoadReport`) instead of aborting on
  the first; sorted order, one Python run per symbol.

### Fixed
- Prove-goal answer check stable under `+`/`*` reordering (`-x + 6 > 0` =
  `6 - x > 0`).
- Fully deterministic solving: `*` factor order via a sorted `IndexMap` (was a
  random-order `HashMap` in release builds).
- A rule with multiple `solve(...) == Param` requirements binds every param.
- Search-loop panics recoverable (`StackOverflow` / `Internal`); an unresolved
  `block(...)` is a load error, not a panic.
- Procedural symbols: a (de)serialize failure is a no-op, and no CWD dependency
  (embedded sympy helper).
- Parser error locations correct with multi-byte content (byte offsets, char
  columns; off-by-one and past-end fixed).
- Logging level filter runs ahead of the async drain — regress at `Info`
  ~85s → ~7s.

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

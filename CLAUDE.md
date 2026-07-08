# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

Release builds are strongly recommended — debug builds are significantly slower.

```bash
cargo build                     # Debug build
cargo build --release           # Release build

# Run binaries
cargo run --release --bin cli -- -c config/cli.yaml
cargo run --release --bin tui -- -c config/tui.yaml
```

## Testing & Linting

```bash
cargo test --workspace             # Run all tests
cargo test -p solver               # Test a single crate
cargo test -p solver -- test_name  # Run a single test
cargo clippy --workspace           # Lint
cargo fmt --all                    # Format
cargo fmt --check                  # Check formatting
```

Snapshot tests use `insta` — run `cargo insta review` to accept/reject snapshot changes.

## Pre-commit checklist

Before `git commit`:

1. **Format**: `cargo +nightly fmt`
2. **Clippy clean**: `cargo clippy --workspace --release --all-targets -- -D warnings`
3. **Tests**: `cargo test --workspace --release` + relevant CLI smoke (`config/cli*.yaml`)
4. **File structure**: `pub` → `pub(super)` → private `fn` (top-down within each `.rs`); tests at the end
5. **Comments**: concise; no doc-essays; Rustdoc in English
6. **`CHANGELOG.md`**: update `[Unreleased]` section if user-visible change
7. **Commit body**: subject + ≤5 lines

## Code Style

### Git Commits

Follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`, etc. Use a scope when relevant, e.g. `feat(solver):`, `fix(parser):`.

### File Structure

Each `.rs` file follows this order:

1. **Imports** (separated by blank lines between groups):
   - `std`
   - Third-party crates
   - Workspace crates (`solver`, `utils`, etc.)
   - `crate` / `super`
2. **Public** constants, traits, functions, structs
3. **Private** constants, traits, functions, structs
4. **`impl` blocks** — `pub` methods first, then private
5. **`#[cfg(test)] mod tests`** — always at the end of the file

### Formatting

`rustfmt.toml`: max_width=100, imports_granularity="Crate", reorder_impl_items=true, normalize/wrap comments.

## Architecture

Laffie is a **symbolic mathematical problem solver** — a rule-based reasoning engine that solves algebra problems (equations, inequalities, transformations, proofs) via bidirectional search with pattern matching and unification.

### Workspace Crates

- **solver** — Core reasoning engine: term representation, rule system, and task-solving algorithm
- **parser** — PEG-based grammar (`peg` crate) for parsing `.sym` (rule/symbol) and `.pbl` (task/problem) files
- **view** — Solution rendering via a `Renderer` trait with Console, HTML, and TUI implementations
- **database** — `redb`-based persistence: tasks (content-addressed `TaskId`) and per-task run history with `SolutionTrace` mirrors
- **utils** — Structured logging (`slog`) and helper utilities
- **cli**, **tui** — Binary applications

> `src/bin/tgbot/` is archived (excluded from the workspace). Reviving it requires a `telegram-bot` crate upgrade and a rewrite onto the new `Db` API.

### Core Data Flow

```
.sym files (symbols/rules) + .pbl files (tasks)
  → Parser (PEG grammar)
  → Solver (bidirectional search: forward from givens + backward from goal)
  → Solution (traced steps + answer)
  → View (Console / HTML / TUI renderer)
```

### Key Concepts (from doc/ru/terminology.md)

- **Term** (`solver::term`) — A formula tree: atoms, variables, parameters, symbols. Represented as `TermBuf` (owned), `SharedTerm` (Arc-wrapped), `TermRef`/`TermMut` (borrows).
- **Symbol** (`solver::term::symbol`) — Domain operator (e.g., `plus`, `equal`, `set`) with custom normalizers. Defined in `.sym` files under `symbols/`.
- **Rule** (`solver::rule`) — A transformation pattern with template, resolution, and requirements. Applied via unification/pattern matching. Results in `Hypothesis` (may have free params) → `GroundedHypothesis` (all params bound) via grounding.
- **Task** (`solver::task`) — A problem with a goal (`find(x)`, `find(x, y)`, `prove(s)`, `transform(s)`), givens, and optional expected answers. Multi-variable find assembles conjunction answers (`x == 3 && y == -2`). Defined in `.pbl` files under `tasks/`.
- **Solver** (`solver::task::solver`) — Bidirectional search engine with cycle budgeting. Forward-chains rules from givens and backward-chains from the goal.

### Domain Files

- `symbols/*.sym` — Symbol definitions and rules (arithmetic, logic, comparison, sets)
- `tasks/**/*.pbl` — Problem definitions organized by category (test, elementary_algebra)
- `config/*.yaml|json` — Per-binary configuration
- `doc/ru/*.md` — Russian-language documentation (source of truth) on terminology, syntax, and architecture; `doc/en/` mirrors it (when populated)

## Detailed Navigation (`claude_docs/`)

The `claude_docs/` directory contains detailed navigation maps for each crate with file listings, key types, line numbers, and dependency graphs:

- [`claude_docs/solver.md`](claude_docs/solver.md) — solver crate: term representation, rule system, solving algorithm
- [`claude_docs/parser.md`](claude_docs/parser.md) — parser crate: PEG grammar, operator precedence, parsing pipeline
- [`claude_docs/view_database_utils.md`](claude_docs/view_database_utils.md) — view, database, utils crates
- [`claude_docs/binaries.md`](claude_docs/binaries.md) — cli, tui binaries

**Use these docs** when navigating unfamiliar parts of the codebase. **Keep them up to date** when making structural changes (adding/removing/renaming files, types, or public API).

## Language

Project documentation and comments are primarily in Russian. Rustdoc comments (`///`, `//!`) must always be written in English.

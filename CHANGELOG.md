# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

//! Persistent storage for tasks and their runs.
//!
//! Backed by `redb` with values encoded as zstd-compressed JSON. Schema
//! evolution relies on `Option`/`#[serde(default)]` fields rather than
//! per-record version tags.

mod codec;
mod db;
mod id;
mod run;
mod task;
mod trace;

pub use db::{Db, RUNS_PER_TASK_LIMIT};
pub use id::{TaskId, compute_task_id, id_from_hex, id_to_hex};
pub use run::{Run, RunStats};
pub use task::Task;
pub use trace::{RuleRef, SolutionTrace, TraceInference, TraceParams, TraceStatus, TraceTerm};

//! Persistent storage for tasks and their runs.
//!
//! Backed by `redb` with values encoded as zstd-compressed JSON. Schema
//! evolution relies on `Option`/`#[serde(default)]` fields rather than
//! per-record version tags.

mod codec;
mod db;
mod id;
mod legacy;
mod run;
mod task;
mod trace;

#[macro_use]
extern crate log;

pub use db::{Db, RUNS_PER_TASK_LIMIT};
pub use id::{TaskId, compute_task_id};
pub use legacy::{TaskDb, TaskRecord};
pub use run::{Run, RunStats};
pub use task::Task;
pub use trace::{RuleRef, SolutionTrace, TraceInference, TraceParams, TraceStatus, TraceTerm};

fn err_handle<T>(result: Result<T, sled::Error>) -> Option<T> {
    result.inspect_err(|e| error!("db error {e}")).ok()
}

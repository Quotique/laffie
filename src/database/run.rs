use chrono::Utc;
use serde_derive::{Deserialize, Serialize};

use solver::{task::Solution, term::TermBuf};

use crate::{
    id::TaskId,
    trace::{SolutionTrace, TraceStatus},
};

/// One execution of a [`Task`](crate::Task).
///
/// `seq` is monotonically increasing per `task_id` and assigned by
/// [`Db::add_run`](crate::Db::add_run); the `(task_id, seq)` pair forms the
/// row key in the `runs` table.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub task_id:    TaskId,
    pub seq:        u64,
    /// Unix epoch seconds at insertion.
    pub created_at: i64,

    pub stats:    RunStats,
    pub solution: SolutionTrace,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunStats {
    pub cycles:      u64,
    pub status:      TraceStatus,
    /// Snapshot of the final answer term, if any. Duplicates
    /// `solution.terms[idx]` when `status == Answer(idx)`; kept here so
    /// quick lookups don't pay the full trace deserialization cost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer:      Option<TermBuf>,
    /// Wall-clock duration, set by the cli around `solve`. `None` when the
    /// producer did not time the run (e.g. `Run::from_solution` before it is
    /// filled in).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl Run {
    /// Builds a [`Run`] from a finished [`Solution`]. `seq` is the placeholder
    /// `0` — [`Db::add_run`](crate::Db::add_run) overwrites it with the next
    /// available sequence number when the run is persisted.
    pub fn from_solution(task_id: TaskId, solution: &Solution) -> Self {
        let trace = SolutionTrace::from(solution);
        let answer = solution.answer().map(|t| (*t).clone());
        let stats = RunStats {
            cycles: solution.cycles() as u64,
            status: trace.status.clone(),
            answer,
            duration_ms: None,
        };
        Run {
            task_id,
            seq: 0,
            created_at: Utc::now().timestamp(),
            stats,
            solution: trace,
        }
    }
}

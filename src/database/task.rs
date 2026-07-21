use chrono::Utc;
use serde_derive::{Deserialize, Serialize};

use solver::{
    task::{Task as SolverTask, TermProps, content_id},
    term::TermBuf,
};

use crate::id::{TaskId, compute_task_id};

/// Persistent description of a problem.
///
/// `id` is content-addressed over `(givens, goal)` (see [`compute_task_id`]),
/// so re-inserting an equivalent task is idempotent. `possible_answers` is
/// validation-only and intentionally **not** part of the id.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id:    TaskId,
    /// Task name (`.pbl` `id "..."`), empty if unset; preserved across the db.
    #[serde(default)]
    pub name:  String,
    pub text:  String,
    pub group: String,

    pub givens:           Vec<TermBuf>,
    pub goal:             TermBuf,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub possible_answers: Vec<TermBuf>,

    #[serde(default)]
    pub hidden:     bool,
    /// Unix epoch seconds at first persistence.
    pub created_at: i64,
}

impl From<&SolverTask> for Task {
    fn from(t: &SolverTask) -> Self {
        let givens: Vec<TermBuf> = t.givens.iter().map(|x| (*x.term).clone()).collect();
        let goal: TermBuf = (*t.goal.term).clone();
        let id = compute_task_id(&givens, &goal);

        Task {
            id,
            name: t.name.clone(),
            text: t.text.clone(),
            group: t.group.clone(),
            givens,
            goal,
            possible_answers: t.possible_answers.clone(),
            hidden: false,
            created_at: Utc::now().timestamp(),
        }
    }
}

impl From<Task> for SolverTask {
    fn from(t: Task) -> Self {
        let givens: Vec<TermProps> = t.givens.into_iter().map(TermProps::from).collect();
        let goal = TermProps::from(t.goal);
        SolverTask {
            id: content_id(&givens, &goal),
            name: t.name,
            text: t.text,
            group: t.group,
            givens,
            goal,
            possible_answers: t.possible_answers,
            subtask_level: 0,
        }
    }
}

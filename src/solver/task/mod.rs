mod builder;
mod goal;
mod props;
mod solution;
mod solver;
mod steps;
mod tracing;

use crate::{
    rule::RuleId,
    term::{SharedTerm, TermBuf},
};
use std::{
    collections::{HashSet, hash_map::DefaultHasher},
    fmt,
    hash::{Hash, Hasher},
    iter::Iterator,
};

pub use builder::TaskBuilder;
pub use goal::{Goal, GoalError, GoalKind};
pub use props::{TermInference, TermProps};
pub use solution::{SharedSolution, Solution, SolutionStatus, SolveError, TermIdx};
pub use solver::{CancelToken, EXECUTION_DEADLINE_DEFAULT, RunControl, Solver, TIME_LIMIT_DEFAULT};
pub use steps::{StepsSource, Visit};
pub use tracing::{Tracer, TracerHub};

/// A problem: what to look for, what is given, how deep in the subtask tree.
///
/// The goal is checked at construction and never again, which is what lets
/// the search assume it is a well-formed `find`/`prove`/`transform`.
#[derive(Debug, Clone)]
pub struct Task {
    pub id:    u64,
    pub name:  String,
    pub text:  String,
    pub group: String,

    pub givens:        Vec<TermProps>,
    pub subtask_level: usize,

    pub possible_answers: Vec<TermBuf>,

    goal: Goal,

    /// A rule that blocked itself on the term a subtask was spawned from stays
    /// blocked inside that subtask.
    blocked_rules: HashSet<RuleId>,
}

impl Task {
    #[inline]
    pub fn goal(&self) -> &Goal {
        &self.goal
    }

    #[inline]
    pub(crate) fn blocked_rules(&self) -> &HashSet<RuleId> {
        &self.blocked_rules
    }

    pub(crate) fn block_rules(&mut self, rules: HashSet<RuleId>) {
        self.blocked_rules = rules;
    }

    pub(crate) fn from_goal(goal: Goal) -> Self {
        Self {
            id: Default::default(),
            name: Default::default(),
            text: Default::default(),
            group: Default::default(),
            givens: Default::default(),
            subtask_level: Default::default(),
            possible_answers: Default::default(),
            blocked_rules: Default::default(),
            goal,
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:x}\n{}{}\n  {}",
            self.id,
            if self.text.is_empty() {
                "".to_owned()
            } else {
                format!("{}\n", self.text)
            },
            self.goal.term(),
            self.givens
                .iter()
                .map(|x| x.term.to_string())
                .collect::<Vec<String>>()
                .join("\n  "),
        )
    }
}

/// Content-addressed `u64` id from a task's givens (order-independent) and
/// goal. Formatting- and location-independent. In-memory dedup key only.
pub fn content_id(givens: &[TermProps], goal: &SharedTerm) -> u64 {
    let mut given_hashes: Vec<u64> = givens
        .iter()
        .map(|g| {
            let mut h = DefaultHasher::new();
            g.term.hash(&mut h);
            h.finish()
        })
        .collect();
    given_hashes.sort_unstable();

    let mut h = DefaultHasher::new();
    given_hashes.hash(&mut h);
    goal.hash(&mut h);
    h.finish()
}

#[cfg(test)]
pub fn parse_task(text: &'static str) -> Task {
    let states = parser::lang::task(text).unwrap();
    let task = parser::TaskParser::from(&states).parse().unwrap();

    // SAFETY: the dev-dependency cycle (solver tests → parser → solver) links a
    // second instance of this crate, so `parser`'s `Task` is a nominally
    // distinct type from ours with an identical layout (same source). Unlike
    // `TermBuf`, `Task` carries a `SharedRule`/solution graph and cannot round
    // trip through serde, so the reinterpret stays. Freshly parsed here, both
    // sides are the same compiler's identical layout of the same definition.
    unsafe { std::mem::transmute::<_, Task>(task) }
}

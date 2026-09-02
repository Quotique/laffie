mod builder;
pub mod goal;

use crate::term::{SharedTerm, TermBuf};
use std::{
    collections::hash_map::DefaultHasher,
    fmt,
    hash::{Hash, Hasher},
    iter::Iterator,
};

pub use builder::TaskBuilder;
pub use goal::{Goal, GoalError, GoalKind};

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

    pub givens:        Vec<SharedTerm>,
    pub subtask_level: usize,

    pub possible_answers: Vec<TermBuf>,

    goal: Goal,
}

impl Task {
    #[inline]
    pub fn goal(&self) -> &Goal {
        &self.goal
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
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join("\n  "),
        )
    }
}

/// Content-addressed `u64` id from a task's givens (order-independent) and
/// goal. Formatting- and location-independent. In-memory dedup key only.
pub fn content_id(givens: &[SharedTerm], goal: &Goal) -> u64 {
    let mut given_hashes: Vec<u64> = givens
        .iter()
        .map(|g| {
            let mut h = DefaultHasher::new();
            g.hash(&mut h);
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

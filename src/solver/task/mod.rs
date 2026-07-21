mod builder;
mod goal;
mod props;
mod solution;
mod solver;
mod steps;
mod tracing;

use crate::term::TermBuf;
use std::{fmt, iter::Iterator};

pub use builder::TaskBuilder;
pub use goal::Goal;
pub use props::{TermInference, TermProps};
pub use solution::{SharedSolution, Solution, SolutionStatus, SolveError, TermIdx};
pub use solver::{CancelToken, EXECUTION_DEADLINE_DEFAULT, RunControl, Solver, TIME_LIMIT_DEFAULT};
pub use steps::{StepsSource, Visit};
pub use tracing::{Tracer, TracerHub};

#[derive(Debug, Clone)]
pub struct Task {
    pub id:    u64,
    pub name:  String,
    pub text:  String,
    pub group: String,

    pub goal:          TermProps,
    pub givens:        Vec<TermProps>,
    pub subtask_level: usize,

    pub possible_answers: Vec<TermBuf>,
}

impl From<TermProps> for Task {
    fn from(value: TermProps) -> Self {
        Self {
            id:               Default::default(),
            name:             Default::default(),
            text:             Default::default(),
            group:            Default::default(),
            goal:             value,
            givens:           Default::default(),
            subtask_level:    Default::default(),
            possible_answers: Default::default(),
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
            self.goal,
            self.givens
                .iter()
                .map(|x| x.term.to_string())
                .collect::<Vec<String>>()
                .join("\n  "),
        )
    }
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

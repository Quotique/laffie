mod builder;
mod goal;
mod props;
mod solution;
mod solver;
mod steps;
mod tracing;

use crate::term::Term;
use std::{fmt, iter::Iterator};

pub use builder::TaskBuilder;
pub use goal::Goal;
pub use props::{TermInference, TermProps};
pub use solution::{SharedSolution, Solution, SolutionStatus, SolveError, TermIdx};
pub use solver::{Solver, EXECUTION_DEADLINE_DEFAULT};
pub use steps::{StepsSource, Visit};
pub use tracing::{Tracer, TracerHub};

#[derive(Debug, Clone)]
pub struct Task {
    pub id:    u64,
    pub text:  String,
    pub group: String,

    pub goal:          TermProps,
    pub givens:        Vec<TermProps>,
    pub subtask_level: usize,

    pub possible_answers: Vec<Term>,
}

impl From<TermProps> for Task {
    fn from(value: TermProps) -> Self {
        Self {
            id:               Default::default(),
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
    let task = parser::TaskParser::with(&states).parse().unwrap();

    unsafe { std::mem::transmute::<_, Task>(task) }
}

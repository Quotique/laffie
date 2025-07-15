mod builder;
mod cache;
mod profiler;
mod props;
mod purpose;
mod solution;
mod solver;
mod tracing;

use crate::term::Term;
use std::{fmt, iter::Iterator};

pub use profiler::{Profiler, ProfilerNode, TaskProfileInfo, TermProfileInfo};

pub use builder::TaskBuilder;
pub use cache::TasksCache;
pub use props::{Cause, TermInference, TermProps};
pub use purpose::Purpose;
pub use solution::Solution;
pub use solver::{Solver, EXECUTION_DEADLINE_DEFAULT};
pub use tracing::{Config as DumperConfig, SolutionTracer, Tracer};

#[derive(Debug, Clone)]
pub struct Task {
    pub id:            u64,
    pub text:          String,
    pub group:         String,
    pub conditions:    Vec<TermProps>,
    pub purpose:       TermProps,
    pub subtask_level: usize,

    pub possible_answers: Vec<Term>,
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
            self.purpose,
            self.conditions
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

mod builder;
mod cache;
mod purpose;
mod solution;
mod tracing;

use crate::term::{Term, TermProps};
use std::{fmt, iter::Iterator};

pub use self::{
    builder::TaskBuilder,
    cache::TasksCache,
    purpose::Purpose,
    solution::{Solution, EXECUTION_DEADLINE_DEFAULT},
    tracing::{
        Config as DumperConfig, Profiler, ProfilerNode, SolutionTracer, TaskProfileInfo,
        TermProfileInfo, Tracer,
    },
};

#[derive(Clone)]
pub struct Task {
    pub id:            u64,
    pub text:          String,
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

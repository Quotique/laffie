mod builder;
mod cache;
mod dump;
mod purpose;
mod solution;

use crate::term::TermProps;
use std::{fmt, iter::Iterator};

pub use self::{
    builder::TaskBuilder,
    cache::TasksCache,
    dump::{Config as DumperConfig, Dumper, DumperSink},
    purpose::Purpose,
    solution::{Solution, SolveStatus},
};

#[derive(Clone)]
pub struct Task {
    pub id:            u64,
    pub conditions:    Vec<TermProps>,
    pub purpose:       TermProps,
    pub subtask_level: usize,
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:x} {}\n  {}",
            self.id,
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

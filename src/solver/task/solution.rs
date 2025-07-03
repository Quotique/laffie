use std::{rc::Rc, sync::Arc};

use super::{Profiler, Purpose, Task, TasksCache, TermProps};
use crate::term::Term;

//#[derive(Debug, Clone, Default)]
#[derive(Debug)]
pub struct Solution {
    pub task:     Task,
    pub purpose:  Purpose,
    pub profiler: Profiler,

    pub cycles: usize,
    pub terms:  Vec<TermProps>,
    pub cache:  Arc<TasksCache>,

    pub answer: Option<usize>,
}

impl Solution {
    pub fn new(task: Task) -> Self {
        let purpose = Purpose::try_from((*task.purpose.term).clone()).unwrap();
        Self {
            task,
            purpose,
            profiler: Default::default(),
            cycles: Default::default(),
            terms: Default::default(),
            cache: Default::default(),
            answer: None,
        }
    }

    pub fn pick_term(&self) -> Option<usize> {
        self.terms
            .iter()
            .filter(|x| !(x.filters.is_replaced() || x.filters.is_purpose()))
            .min_by_key(|x| x.filters.weight)
            .map(|x| x.inference.id)
    }

    #[inline]
    pub fn current_cycles(&self) -> usize {
        self.cycles
    }

    #[inline]
    pub fn increment_cycles(&mut self) {
        self.cycles += 1;
    }

    #[inline]
    pub fn answer(&self) -> Option<Rc<Term>> {
        self.answer.map(|i| self.terms[i].term.clone())
    }

    pub fn validate_answer(&self) -> bool {
        if self.task.possible_answers.is_empty() {
            return true;
        }

        if let Some(answer) = self.answer() {
            if self
                .task
                .possible_answers
                .iter()
                .any(|x| x == answer.as_ref())
            {
                return true;
            }
            // TODO: есть проблема с неправильным преобразованием дерева, что приводит к
            // некорректному прямому сравнению дерева.
            return self
                .task
                .possible_answers
                .iter()
                .any(|x| x.to_string() == answer.to_string());
        }
        false
    }
}

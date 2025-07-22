use std::{
    collections::HashMap,
    error,
    ops::{Index, IndexMut},
    rc::Rc,
};

use bincode::{Decode, Encode};
use derive_more::Display;
use itertools::Itertools;

use super::{Purpose, Task, TermProps};
use crate::term::SharedTerm;

pub const STACK_SIZE: usize = 2048;

pub type SharedSolution = Rc<Solution>;
pub type TermIdx = usize;

#[derive(Debug, Display, Clone, Copy, Encode, Decode, PartialEq, Eq)]
pub enum SolveError {
    StackOverflow,
    MaxSubtaskLevelExceed,
    NoConditions,
    NoSolutionsFound,
    ExecutionDeadline,
}

#[derive(Debug, Display, Default, Clone, Copy, Encode, Decode)]
pub enum SolutionStatus {
    #[default]
    NotDone,
    Answer(usize),
    Err(SolveError),
}

#[derive(Debug)]
pub struct Solution {
    pub task:    Task,
    pub purpose: Purpose,

    pub status:      SolutionStatus,
    pub start_cycle: usize,
    pub end_cycle:   usize,

    pub main_index:       HashMap<SharedTerm, usize>,
    pub purpose_index:    HashMap<SharedTerm, usize>,
    pub terms:            Vec<TermProps>,
    unproven_terms_count: usize,
}

impl Solution {
    pub fn new(task: Task) -> Self {
        let purpose = Purpose::try_from((*task.purpose.term).clone()).unwrap();

        let mut solution = Self {
            task,
            purpose,
            start_cycle: Default::default(),
            end_cycle: Default::default(),
            main_index: Default::default(),
            purpose_index: Default::default(),
            terms: Default::default(),
            status: Default::default(),
            unproven_terms_count: Default::default(),
        };
        let conditions = solution.task.conditions.clone();
        for i in conditions.into_iter() {
            let _ = solution.add_term(i);
        }
        let _ = solution.add_term(solution.purpose.term().clone());

        trace!(target: "subtask", "Subtask: {}, [{}]",
            solution.purpose, solution.task.conditions.iter().format(", ")
        );
        solution
    }

    #[inline]
    pub fn cycles(&self) -> usize {
        self.end_cycle - self.start_cycle
    }

    pub fn add_term(&mut self, mut term: TermProps) -> Result<TermIdx, SolveError> {
        if self.terms.len() - self.unproven_terms_count + 1 > STACK_SIZE {
            return Err(SolveError::StackOverflow);
        }
        let id = self.terms.len();
        term.id = id;

        if !term.inference.is_proven() {
            self.unproven_terms_count += 1;
            self.terms.push(term);
            return Ok(id);
        }

        let index = if term.filters.is_purpose() {
            &mut self.purpose_index
        } else {
            &mut self.main_index
        };
        if let Some(id) = index.get(&term.term) {
            return Ok(*id);
        }
        index.insert(term.term.clone(), id);

        self.terms.push(term);
        Ok(id)
    }

    pub fn pick_next(&self) -> Option<TermIdx> {
        self.terms
            .iter()
            .filter(|x| x.inference.is_proven())
            .filter(|x| !x.filters.is_replaced())
            .min_by_key(|x| x.filters.weight)
            .map(|x| x.id)
    }

    pub fn pick_purpose_term(&self) -> Option<TermIdx> {
        self.terms
            .iter()
            .filter(|x| x.inference.is_proven())
            .filter(|x| !x.filters.is_replaced() && x.filters.is_purpose())
            .min_by_key(|x| x.filters.weight)
            .map(|x| x.id)
    }

    #[inline]
    pub fn answer(&self) -> Option<SharedTerm> {
        if let SolutionStatus::Answer(i) = self.status {
            return Some(self.terms[i].term.clone());
        }
        None
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

impl Index<TermIdx> for Solution {
    type Output = TermProps;

    fn index(&self, index: TermIdx) -> &Self::Output {
        &self.terms[index]
    }
}

impl IndexMut<TermIdx> for Solution {
    fn index_mut(&mut self, index: TermIdx) -> &mut Self::Output {
        &mut self.terms[index]
    }
}

impl error::Error for SolveError {}

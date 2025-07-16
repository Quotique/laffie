use std::{
    cell::RefCell,
    collections::{HashSet, VecDeque},
    rc::Rc,
    sync::Arc,
};

use super::{Purpose, Task, TasksCache, TermProps};
use crate::term::{SharedTerm, Term};

pub type SharedSolution = Rc<Solution>;

#[derive(Debug)]
pub struct Solution {
    pub task:    Task,
    pub purpose: Purpose,

    pub cycles: usize,
    pub terms:  Vec<TermProps>,
    pub cache:  Arc<TasksCache>,

    pub answer: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Steps {
    solution:    SharedSolution,
    terms_queue: Vec<usize>,

    subtasks: VecDeque<Steps>,
    rendered: Arc<RefCell<HashSet<Term>>>,
}

impl From<SharedSolution> for Steps {
    fn from(solution: SharedSolution) -> Self {
        // TODO: no answer
        let answer_idx = solution.answer.unwrap();
        let mut terms_queue: Vec<usize> = vec![answer_idx];

        while let Some(ref parent) = solution.terms[*terms_queue.last().unwrap()]
            .inference
            .parent_id()
        {
            terms_queue.push(*parent);
        }
        Self {
            solution,
            terms_queue,
            subtasks: Default::default(),
            rendered: Default::default(),
        }
    }
}

impl Iterator for Steps {
    type Item = SharedTerm;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let subtask_empty = self.subtasks.is_empty();
            while let Some(subtask) = self.subtasks.front_mut() {
                if let Some(term) = subtask.next() {
                    return Some(term);
                }
                self.subtasks.pop_front();
            }

            if !subtask_empty {
                let id = self.terms_queue.pop().unwrap();
                return Some(self.solution.terms[id].term.clone());
            }

            if let Some(next_id) = self.terms_queue.last() {
                for r in self.solution.terms[*next_id]
                    .inference
                    .requirements()
                    .iter()
                    .flat_map(|x| x.iter())
                    .filter(|x| !self.rendered.borrow().contains(&x.task.purpose.term))
                {
                    self.rendered
                        .borrow_mut()
                        .insert(r.task.purpose.term.as_ref().clone());
                    let mut steps = Steps::from(r.clone());
                    steps.rendered = self.rendered.clone();
                    self.subtasks.push_back(steps);
                }
            } else {
                return None;
            }
        }
    }
}

impl Solution {
    pub fn new(task: Task) -> Self {
        let purpose = Purpose::try_from((*task.purpose.term).clone()).unwrap();
        Self {
            task,
            purpose,
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
            .map(|x| x.id)
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
    pub fn answer(&self) -> Option<SharedTerm> {
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

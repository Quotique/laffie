use std::{
    cell::RefCell,
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use super::{SharedSolution, Task, solution::SolutionStatus};
use crate::term::{SharedTerm, TermBuf};

pub trait StepsSource {
    fn steps(&self) -> Steps;
}

#[derive(Debug, Clone)]
pub enum Visit {
    Subtask(Box<Task>),
    Term(SharedTerm),
    Answer(SharedTerm),
}

#[derive(Clone, Debug)]
pub struct Steps {
    visit_task:  bool,
    solution:    SharedSolution,
    terms_queue: Vec<usize>,

    subtasks: VecDeque<Steps>,
    rendered: Arc<RefCell<HashSet<TermBuf>>>,
}

impl StepsSource for SharedSolution {
    fn steps(&self) -> Steps {
        Steps::from(self.clone())
    }
}

impl From<SharedSolution> for Steps {
    fn from(solution: SharedSolution) -> Self {
        let terms_queue = match solution.status {
            SolutionStatus::Answer(answer_idx) => {
                let mut terms_queue: Vec<usize> = vec![answer_idx];

                while let Some(ref parent) =
                    solution[*terms_queue.last().unwrap()].inference.parent_id()
                {
                    terms_queue.push(*parent);
                }
                terms_queue
            }
            _ => solution
                .terms
                .iter()
                .rev()
                .enumerate()
                .filter(|(_, x)| x.is_proven())
                .map(|(n, _)| n)
                .collect(),
        };

        Self {
            visit_task: true,
            solution,
            terms_queue,
            subtasks: Default::default(),
            rendered: Default::default(),
        }
    }
}

impl Iterator for Steps {
    type Item = Visit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.visit_task {
            self.visit_task = false;
            return Some(Visit::Subtask(Box::new(self.solution.task.clone())));
        }
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
                return Some(if self.terms_queue.is_empty() {
                    Visit::Answer(self.solution[id].term.clone())
                } else {
                    Visit::Term(self.solution[id].term.clone())
                });
            }

            let next_id = self.terms_queue.last()?;
            for r in self.solution[*next_id]
                .inference
                .requirements()
                .filter(|x| !self.rendered.borrow().contains(&x.task.goal.term))
            {
                self.rendered
                    .borrow_mut()
                    .insert(r.task.goal.term.as_ref().clone());
                let mut steps = Steps::from(r.clone());
                steps.rendered = self.rendered.clone();
                self.subtasks.push_back(steps);
            }
            if self.subtasks.is_empty() {
                let id = self.terms_queue.pop().unwrap();
                return Some(if self.terms_queue.is_empty() {
                    Visit::Answer(self.solution[id].term.clone())
                } else {
                    Visit::Term(self.solution[id].term.clone())
                });
            }
        }
    }
}

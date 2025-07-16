use std::{
    cell::RefCell,
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use super::{solution::SolutionStatus, SharedSolution};
use crate::term::{SharedTerm, Term};

#[derive(Clone, Debug)]
pub struct Steps {
    solution:    SharedSolution,
    terms_queue: Vec<usize>,

    subtasks: VecDeque<Steps>,
    rendered: Arc<RefCell<HashSet<Term>>>,
}

impl From<SharedSolution> for Steps {
    fn from(solution: SharedSolution) -> Self {
        let answer_idx = match solution.status {
            SolutionStatus::Answer(i) => i,
            // TODO: no answer
            _ => unimplemented!(""),
        };

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

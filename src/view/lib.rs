mod console;
mod html;
mod tui;

use std::{cell::RefCell, collections::HashSet, convert::TryFrom, fmt, sync::Arc};

pub use console::Console;
pub use html::Html;
pub use tui::Tui;

use solver::{
    task::{Purpose, Solver},
    term::{Term, TermProps},
};

pub trait Renderer {
    fn display_purpose(&mut self, subtask_level: usize, purpose: &Purpose) -> fmt::Result;

    fn display_term(&mut self, subtask_level: usize, term: &TermProps) -> fmt::Result;

    fn display_answer(
        &mut self,
        purpose: &Purpose,
        answer: Option<&Term>,
        status: &Solver,
    ) -> fmt::Result;

    fn dump_frame(&mut self, _frame: &[TermProps]) -> fmt::Result {
        Ok(())
    }
}

pub struct View<'a> {
    solution: &'a Solver,
    rendered: Arc<RefCell<HashSet<Term>>>,
}

impl<'a> TryFrom<&'a Solver> for View<'a> {
    type Error = eyre::Error;

    fn try_from(solution: &'a Solver) -> eyre::Result<Self> {
        Ok(Self {
            solution,
            rendered: Default::default(),
        })
    }
}

impl View<'_> {
    fn display_purpose(
        &self,
        purpose: &Purpose,
        answer: &Term,
        subtask_level: usize,
        renderer: &mut dyn Renderer,
    ) -> fmt::Result {
        renderer.display_purpose(subtask_level, purpose)?;
        match purpose {
            Purpose::Find(_) | Purpose::Transform(_) => {}
            Purpose::Proof(_) => {
                let answer_idx = self
                    .solution
                    .terms
                    .iter()
                    .filter(|x| x.is_purpose)
                    .enumerate()
                    .find(|(_, x)| x.term.as_ref() == answer)
                    .map(|(id, _)| id);
                if let Some(idx) = answer_idx {
                    self.display_frame(&self.solution.terms, idx, subtask_level, renderer)?;
                }
            }
        }
        Ok(())
    }

    fn display_frame(
        &self,
        frame: &[TermProps],
        answer_idx: usize,
        subtask_level: usize,
        renderer: &mut dyn Renderer,
    ) -> fmt::Result {
        let mut trace: Vec<usize> = vec![answer_idx];

        while let Some(parent) = frame[*trace.last().unwrap()].parent {
            trace.push(parent);
        }

        while let Some(id) = trace.pop() {
            for r in &frame[id].requirements {
                if self.rendered.borrow_mut().insert(r.as_ref().clone()) {
                    if let Some(solution) = self.solution.cache.status(r).and_then(|x| x.solver()) {
                        View {
                            solution: &solution,
                            rendered: self.rendered.clone(),
                        }
                        .display_impl(renderer)?;
                    }
                }
            }
            if !trace.is_empty() {
                renderer.display_term(subtask_level, &frame[id])?;
            }
        }
        Ok(())
    }

    pub fn display_impl(&self, renderer: &mut dyn Renderer) -> fmt::Result {
        if let Some(a) = self.solution.answer {
            self.display_purpose(
                &self.solution.purpose,
                &self.solution.terms[a].term,
                self.solution.task.subtask_level,
                renderer,
            )?;
            self.display_frame(
                &self.solution.terms,
                a,
                self.solution.task.subtask_level,
                renderer,
            )?;
        } else {
            renderer.dump_frame(&self.solution.terms)?;
        }
        if self.solution.task.subtask_level == 0 {
            renderer.display_answer(
                &self.solution.purpose,
                self.solution
                    .answer
                    .map(|x| self.solution.terms[x].term.as_ref()),
                self.solution,
            )?;
        }
        Ok(())
    }
}

impl fmt::Display for View<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display_impl(&mut Console { output: f })
    }
}

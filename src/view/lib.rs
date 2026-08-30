#[cfg(feature = "console")]
mod console;
#[cfg(feature = "html")]
mod html;
#[cfg(feature = "tui")]
mod tui;

use std::{cell::RefCell, collections::HashSet, convert::TryFrom, fmt, sync::Arc};

#[cfg(feature = "console")]
pub use console::Console;
#[cfg(feature = "html")]
pub use html::Html;
#[cfg(feature = "tui")]
pub use tui::Tui;

use solver::{
    task::{Goal, Solution, SolutionStatus, TermProps},
    term::{SharedTerm, TermBuf},
};

pub trait Renderer {
    fn display_goal(&mut self, subtask_level: usize, goal: &Goal) -> fmt::Result;

    fn display_term(&mut self, subtask_level: usize, term: &TermProps) -> fmt::Result;

    fn display_answer(
        &mut self,
        goal: &Goal,
        answer: Option<SharedTerm>,
        status: &Solution,
    ) -> fmt::Result;

    fn dump_frame(&mut self, _frame: &[TermProps]) -> fmt::Result {
        Ok(())
    }
}

pub struct View<'a> {
    solution: &'a Solution,
    rendered: Arc<RefCell<HashSet<TermBuf>>>,
}

impl<'a> TryFrom<&'a Solution> for View<'a> {
    type Error = eyre::Error;

    fn try_from(solution: &'a Solution) -> eyre::Result<Self> {
        Ok(Self {
            solution,
            rendered: Default::default(),
        })
    }
}

impl View<'_> {
    fn display_goal(
        &self,
        goal: &Goal,
        answer: &TermBuf,
        subtask_level: usize,
        renderer: &mut dyn Renderer,
    ) -> fmt::Result {
        renderer.display_goal(subtask_level, goal)?;
        match goal {
            Goal::Find(_) | Goal::Transform(_) => {}
            Goal::Prove(_) => {
                let answer_idx = self
                    .solution
                    .terms
                    .iter()
                    .filter(|x| x.filters.is_goal())
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

        while let Some(parent) = frame[*trace.last().unwrap()].inference.parent_id() {
            trace.push(parent);
        }

        while let Some(id) = trace.pop() {
            for r in frame[id].inference.requirements() {
                if self
                    .rendered
                    .borrow_mut()
                    .insert(r.task.goal().term.as_ref().clone())
                {
                    View {
                        solution: r.as_ref(),
                        rendered: self.rendered.clone(),
                    }
                    .display_impl(renderer)?;
                }
            }
            if !trace.is_empty() {
                renderer.display_term(subtask_level, &frame[id])?;
            }
        }
        Ok(())
    }

    pub fn display_impl(&self, renderer: &mut dyn Renderer) -> fmt::Result {
        if let SolutionStatus::Answer(a) = self.solution.status {
            self.display_goal(
                self.solution.goal(),
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
            renderer.display_answer(self.solution.goal(), self.solution.answer(), self.solution)?;
        }
        Ok(())
    }
}

#[cfg(feature = "console")]
impl fmt::Display for View<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display_impl(&mut Console { output: f })
    }
}

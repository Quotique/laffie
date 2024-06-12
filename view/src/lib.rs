mod console;
mod html;

use std::{cell::RefCell, collections::HashSet, convert::TryFrom, fmt, sync::Arc};

pub use console::Console;
pub use html::Html;

use mcore::{
    task::{Frame, Purpose, Solution, SolveStatus, TasksCache},
    term::{Term, TermProps},
};

pub trait Renderer {
    fn display_purpose(&mut self, subtask_level: usize, purpose: &Purpose) -> fmt::Result;

    fn display_term(&mut self, subtask_level: usize, term: &TermProps) -> fmt::Result;

    fn display_answer(
        &mut self,
        purpose: &Purpose,
        answer: Option<&Term>,
        status: &SolveStatus,
    ) -> fmt::Result;

    fn dump_frame(&mut self, _frame: &Frame) -> fmt::Result {
        Ok(())
    }
}

pub struct View<'a> {
    solution: &'a Solution,
    subtasks: Arc<TasksCache>,
    rendered: Arc<RefCell<HashSet<Term>>>,
}

impl<'a> TryFrom<&'a Solution> for View<'a> {
    type Error = eyre::Error;

    fn try_from(solution: &'a Solution) -> eyre::Result<Self> {
        Ok(Self {
            solution,
            subtasks: solution
                .cache
                .clone()
                .ok_or_else(|| eyre::eyre!("missing cache"))?,
            rendered: Default::default(),
        })
    }
}

impl<'a> View<'a> {
    fn display_purpose(
        &self,
        purpose: &Purpose,
        answer: &Term,
        subtask_level: usize,
        renderer: &mut dyn Renderer,
    ) -> fmt::Result {
        renderer.display_purpose(subtask_level, purpose)?;
        match purpose {
            Purpose::Find(_) => {}
            Purpose::Proof(s) | Purpose::Transform(s) => {
                let answer_idx = s
                    .iter()
                    .enumerate()
                    .find(|(_, x)| x.term.as_ref() == answer)
                    .map(|(id, _)| id);
                if let Some(idx) = answer_idx {
                    self.display_frame(s, idx, subtask_level, renderer)?;
                }
            }
        }
        Ok(())
    }

    fn display_frame(
        &self,
        frame: &Frame,
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
                    if let Some(solution) = self.subtasks.status(r).and_then(|x| x.solution()) {
                        View {
                            solution: &solution,
                            subtasks: self.subtasks.clone(),
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
                &self.solution.stack[a].term,
                self.solution.task.subtask_level,
                renderer,
            )?;
            self.display_frame(
                &self.solution.stack,
                a,
                self.solution.task.subtask_level,
                renderer,
            )?;
        } else {
            renderer.dump_frame(&self.solution.stack)?;
        }
        if self.solution.task.subtask_level == 0 {
            renderer.display_answer(
                &self.solution.purpose,
                self.solution
                    .answer
                    .map(|x| self.solution.stack[x].term.as_ref()),
                &self.solution.perf_stats,
            )?;
        }
        Ok(())
    }
}

impl<'a> fmt::Display for View<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display_impl(&mut Console { output: f })
    }
}

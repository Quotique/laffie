mod console;
mod html;

use std::{cell::RefCell, collections::HashSet, convert::TryFrom, fmt, sync::Arc};

pub use console::Console;
pub use html::Html;

use mcore::{
    problem::{Frame, ProblemsCache, Solution, SolveStatus, Target},
    statement::{MarkedStatement, Statement},
};

pub trait Renderer {
    fn display_target(&mut self, subproblem_level: usize, target: &Target) -> fmt::Result;

    fn display_statement(
        &mut self,
        subproblem_level: usize,
        statement: &MarkedStatement,
    ) -> fmt::Result;

    fn display_answer(
        &mut self,
        target: &Target,
        answer: Option<&Statement>,
        status: &SolveStatus,
    ) -> fmt::Result;

    fn dump_frame(&mut self, _frame: &Frame) -> fmt::Result {
        Ok(())
    }
}

pub struct View<'a> {
    solution:    &'a Solution,
    subproblems: Arc<ProblemsCache>,
    rendered:    Arc<RefCell<HashSet<Statement>>>,
}

impl<'a> TryFrom<&'a Solution> for View<'a> {
    type Error = eyre::Error;

    fn try_from(solution: &'a Solution) -> eyre::Result<Self> {
        Ok(Self {
            solution,
            subproblems: solution
                .cache
                .clone()
                .ok_or_else(|| eyre::eyre!("missing cache"))?,
            rendered: Default::default(),
        })
    }
}

impl<'a> View<'a> {
    fn display_target(
        &self,
        target: &Target,
        answer: &Statement,
        subproblem_level: usize,
        renderer: &mut dyn Renderer,
    ) -> fmt::Result {
        renderer.display_target(subproblem_level, target)?;
        match target {
            Target::Find(_) => {}
            Target::Proof(s) | Target::Transform(s) => {
                let answer_idx = s
                    .iter()
                    .enumerate()
                    .find(|(_, x)| x.statement.as_ref() == answer)
                    .map(|(id, _)| id);
                if let Some(idx) = answer_idx {
                    self.display_frame(s, idx, subproblem_level, renderer)?;
                }
            }
        }
        Ok(())
    }

    fn display_frame(
        &self,
        frame: &Frame,
        answer_idx: usize,
        subproblem_level: usize,
        renderer: &mut dyn Renderer,
    ) -> fmt::Result {
        let mut trace: Vec<usize> = vec![answer_idx];

        while let Some(parent) = frame[*trace.last().unwrap()].parent {
            trace.push(parent);
        }

        while let Some(id) = trace.pop() {
            for r in &frame[id].requirements {
                if self.rendered.borrow_mut().insert(r.as_ref().clone()) {
                    if let Some(solution) = self.subproblems.status(r).and_then(|x| x.solution()) {
                        View {
                            solution:    &solution,
                            subproblems: self.subproblems.clone(),
                            rendered:    self.rendered.clone(),
                        }
                        .display_impl(renderer)?;
                    }
                }
            }
            if !trace.is_empty() {
                renderer.display_statement(subproblem_level, &frame[id])?;
            }
        }
        Ok(())
    }

    fn display_impl(&self, renderer: &mut dyn Renderer) -> fmt::Result {
        if let Some(a) = self.solution.answer {
            self.display_target(
                &self.solution.target,
                &self.solution.stack[a].statement,
                self.solution.problem.subproblem_level,
                renderer,
            )?;
            self.display_frame(
                &self.solution.stack,
                a,
                self.solution.problem.subproblem_level,
                renderer,
            )?;
        } else {
            renderer.dump_frame(&self.solution.stack)?;
        }
        if self.solution.problem.subproblem_level == 0 {
            renderer.display_answer(
                &self.solution.target,
                self.solution
                    .answer
                    .map(|x| self.solution.stack[x].statement.as_ref()),
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

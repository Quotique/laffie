use std::fmt;

use colored::*;

use itertools::Itertools;

use solver::{
    task::{Goal, Solution, TermProps},
    term::SharedTerm,
};

use crate::Renderer;

pub struct Console<'a, 'b> {
    pub output: &'a mut fmt::Formatter<'b>,
}

impl Renderer for Console<'_, '_> {
    fn display_goal(&mut self, subtask_level: usize, goal: &Goal) -> fmt::Result {
        writeln!(self.output, "{}{goal}", "  ".repeat(subtask_level))
    }

    fn display_term(&mut self, subtask_level: usize, term: &TermProps) -> fmt::Result {
        writeln!(
            self.output,
            "{}=> {}{}",
            "  ".repeat(subtask_level),
            term.term.to_string().bold().yellow(),
            if term.inference.requirements().next().is_none() {
                Default::default()
            } else {
                format!(
                    " needed: [{}]",
                    term.inference
                        .requirements()
                        .map(|x| &x.task.goal.term)
                        .format(", ")
                )
            }
        )
    }

    fn display_answer(
        &mut self,
        goal: &Goal,
        answer: Option<SharedTerm>,
        status: &Solution,
    ) -> fmt::Result {
        if let Some(answer) = answer.as_ref() {
            writeln!(
                self.output,
                "{} {}",
                match goal {
                    Goal::Find(_) | Goal::Transform(_) => {
                        format!("{} {answer}", "Answer:".green()).bold()
                    }
                    Goal::Proof(_) => {
                        "PROOFED!".bold().green()
                    }
                },
                format!("[{} cycles]", status.cycles()).yellow()
            )
        } else {
            writeln!(
                self.output,
                "{} {}",
                "NOT SOLVED!".bold().blink().red(),
                format!("[{} cycles]", status.cycles()).yellow()
            )
        }
    }

    fn dump_frame(&mut self, frame: &[TermProps]) -> fmt::Result {
        for (i, s) in frame.iter().enumerate() {
            writeln!(self.output, "{i} {} {:?}", s.term, s.inference.parent_id())?;
        }
        Ok(())
    }
}

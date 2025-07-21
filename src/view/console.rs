use std::fmt;

use colored::*;

use itertools::Itertools;

use solver::{
    task::{Purpose, Solution, TermProps},
    term::SharedTerm,
};

use crate::Renderer;

pub struct Console<'a, 'b> {
    pub output: &'a mut fmt::Formatter<'b>,
}

impl Renderer for Console<'_, '_> {
    fn display_purpose(&mut self, subtask_level: usize, purpose: &Purpose) -> fmt::Result {
        writeln!(self.output, "{}{purpose}", "  ".repeat(subtask_level))
    }

    fn display_term(&mut self, subtask_level: usize, term: &TermProps) -> fmt::Result {
        writeln!(
            self.output,
            "{}=> {}{}",
            "  ".repeat(subtask_level),
            term.term.to_string().bold().yellow(),
            if term
                .inference
                .requirements()
                .map(|x| x.is_empty())
                .unwrap_or(true)
            {
                Default::default()
            } else {
                format!(
                    " needed: [{}]",
                    term.inference
                        .requirements()
                        .iter()
                        .flat_map(|x| x.iter())
                        .map(|x| &x.task.purpose.term)
                        .format(", ")
                )
            }
        )
    }

    fn display_answer(
        &mut self,
        purpose: &Purpose,
        answer: Option<SharedTerm>,
        status: &Solution,
    ) -> fmt::Result {
        if let Some(answer) = answer.as_ref() {
            writeln!(
                self.output,
                "{} {}",
                match purpose {
                    Purpose::Find(_) | Purpose::Transform(_) => {
                        format!("{} {answer}", "Answer:".green()).bold()
                    }
                    Purpose::Proof(_) => {
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

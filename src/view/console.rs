use std::fmt;

use colored::*;

use mcore::{
    task::{Purpose, Solution},
    term::{Term, TermProps},
};
use utils::VecDisplay;

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
            if term.requirements.is_empty() {
                Default::default()
            } else {
                format!(" needed: [{}]", VecDisplay(&term.requirements))
            }
        )
    }

    fn display_answer(
        &mut self,
        purpose: &Purpose,
        answer: Option<&Term>,
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
                format!("[{} cycles]", status.current_cycles()).yellow()
            )
        } else {
            writeln!(
                self.output,
                "{} {}",
                "NOT SOLVED!".bold().blink().red(),
                format!("[{} cycles]", status.current_cycles()).yellow()
            )
        }
    }

    fn dump_frame(&mut self, frame: &[TermProps]) -> fmt::Result {
        for (i, s) in frame.iter().enumerate() {
            writeln!(self.output, "{i} {} {:?}", s.term, s.parent)?;
        }
        Ok(())
    }
}

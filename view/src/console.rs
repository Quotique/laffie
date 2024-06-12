use std::fmt;

use colored::*;

use mcore::{
    task::{Frame, Purpose, SolveStatus},
    term::{Term, TermProps},
    utils::VecDisplay,
};

use crate::Renderer;

pub struct Console<'a, 'b> {
    pub output: &'a mut fmt::Formatter<'b>,
}

impl<'a, 'b> Renderer for Console<'a, 'b> {
    fn display_purpose(&mut self, subtask_level: usize, purpose: &Purpose) -> fmt::Result {
        writeln!(self.output, "{}{purpose}", "  ".repeat(subtask_level))
    }

    fn display_term(&mut self, subtask_level: usize, term: &TermProps) -> fmt::Result {
        writeln!(
            self.output,
            "{}=> {} from: {}",
            "  ".repeat(subtask_level),
            term.term.to_string().bold().yellow(),
            VecDisplay(&term.requirements),
        )
    }

    fn display_answer(
        &mut self,
        purpose: &Purpose,
        answer: Option<&Term>,
        status: &SolveStatus,
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
                format!(
                    "[{} cycles, {}ms]",
                    status.cycles_count, status.absolute_time
                )
                .yellow()
            )
        } else {
            writeln!(self.output, "{}", "NOT SOLVED!".bold().blink().red())
        }
    }

    fn dump_frame(&mut self, frame: &Frame) -> fmt::Result {
        for (i, s) in frame.iter().enumerate() {
            writeln!(self.output, "{i} {} {:?}", s.term, s.parent)?;
        }
        Ok(())
    }
}

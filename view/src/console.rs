use std::fmt;

use colored::*;

use mcore::{
    problem::{Frame, SolveStatus, Target},
    statement::{MarkedStatement, Statement},
    utils::VecDisplay,
};

use crate::Renderer;

pub struct Console<'a, 'b> {
    pub output: &'a mut fmt::Formatter<'b>,
}

impl<'a, 'b> Renderer for Console<'a, 'b> {
    fn display_target(&mut self, subproblem_level: usize, target: &Target) -> fmt::Result {
        writeln!(self.output, "{}{target}", "  ".repeat(subproblem_level))
    }

    fn display_statement(
        &mut self,
        subproblem_level: usize,
        statement: &MarkedStatement,
    ) -> fmt::Result {
        writeln!(
            self.output,
            "{}=> {} from: {}",
            "  ".repeat(subproblem_level),
            statement.statement.to_string().bold().yellow(),
            VecDisplay(&statement.requirements),
        )
    }

    fn display_answer(
        &mut self,
        target: &Target,
        answer: Option<&Statement>,
        status: &SolveStatus,
    ) -> fmt::Result {
        if let Some(answer) = answer.as_ref() {
            writeln!(
                self.output,
                "{} {}",
                match target {
                    Target::Find(_) | Target::Transform(_) => {
                        format!("{} {answer}", "Answer:".green()).bold()
                    }
                    Target::Proof(_) => {
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
            writeln!(self.output, "{i} {} {:?}", s.statement, s.parent)?;
        }
        Ok(())
    }
}

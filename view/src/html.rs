use std::fmt;

use html_escape::encode_text;

use mcore::{
    problem::{Frame, SolveStatus, Target},
    statement::{MarkedStatement, Statement},
    utils::VecDisplay,
};

use crate::Renderer;

pub struct Html<'a> {
    pub output: &'a mut dyn fmt::Write,
}

impl<'a> Renderer for Html<'a> {
    fn display_target(&mut self, subproblem_level: usize, target: &Target) -> fmt::Result {
        self.output.write_str(&format!(
            "{}{}\n",
            "  ".repeat(subproblem_level),
            encode_text(&target.to_string())
        ))
    }

    fn display_statement(
        &mut self,
        subproblem_level: usize,
        statement: &MarkedStatement,
    ) -> fmt::Result {
        self.output.write_str(&format!(
            "{}=> <b>{}</b>\n",
            "  ".repeat(subproblem_level),
            encode_text(&statement.statement.to_string()),
        ))
    }

    fn display_answer(
        &mut self,
        target: &Target,
        answer: Option<&Statement>,
        status: &SolveStatus,
    ) -> fmt::Result {
        if let Some(answer) = answer.as_ref() {
            self.output.write_str(&format!(
                "{} {}\n",
                match target {
                    Target::Find(_) | Target::Transform(_) => {
                        format!("<b>Answer:</b> {}", encode_text(&answer.to_string()))
                    }
                    Target::Proof(_) => {
                        "<b>PROOFED!</b>".to_owned()
                    }
                },
                format!(
                    "[{} cycles, {}ms]",
                    status.cycles_count, status.absolute_time
                )
            ))
        } else {
            self.output.write_str("<b>NOT SOLVED!</b>\n")
        }
    }

    fn dump_frame(&mut self, frame: &Frame) -> fmt::Result {
        for (i, s) in frame.iter().enumerate() {
            self.output
                .write_str(&format!("{i} {} {:?}\n", s.statement, s.parent))?;
        }
        Ok(())
    }
}

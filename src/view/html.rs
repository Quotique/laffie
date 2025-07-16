use std::fmt;

use html_escape::encode_text;

use solver::{
    task::{Purpose, Solution, TermProps},
    term::SharedTerm,
};

use crate::Renderer;

pub struct Html<'a> {
    pub output: &'a mut dyn fmt::Write,
}

impl Renderer for Html<'_> {
    fn display_purpose(&mut self, subtask_level: usize, purpose: &Purpose) -> fmt::Result {
        self.output.write_str(&format!(
            "{}{}\n",
            "  ".repeat(subtask_level),
            encode_text(&purpose.to_string())
        ))
    }

    fn display_term(&mut self, subtask_level: usize, term: &TermProps) -> fmt::Result {
        self.output.write_str(&format!(
            "{}=> <b>{}</b>\n",
            "  ".repeat(subtask_level),
            encode_text(&term.term.to_string()),
        ))
    }

    fn display_answer(
        &mut self,
        purpose: &Purpose,
        answer: Option<SharedTerm>,
        status: &Solution,
    ) -> fmt::Result {
        if let Some(answer) = answer.as_ref() {
            self.output.write_str(&format!(
                "{} {}\n",
                match purpose {
                    Purpose::Find(_) | Purpose::Transform(_) => {
                        format!("<b>Answer:</b> {}", encode_text(&answer.to_string()))
                    }
                    Purpose::Proof(_) => {
                        "<b>PROOFED!</b>".to_owned()
                    }
                },
                format_args!("[{} cycles, ]", status.current_cycles())
            ))
        } else {
            self.output.write_str("<b>NOT SOLVED!</b>\n")
        }
    }

    fn dump_frame(&mut self, frame: &[TermProps]) -> fmt::Result {
        for (i, s) in frame.iter().enumerate() {
            self.output
                .write_str(&format!("{i} {} {:?}\n", s.term, s.inference.parent_id()))?;
        }
        Ok(())
    }
}

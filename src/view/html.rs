use std::fmt;

use html_escape::encode_text;

use solver::{
    engine::{Solution, TermProps},
    task::{Answer, Goal, GoalKind},
};

use crate::Renderer;

pub struct Html<'a> {
    pub output: &'a mut dyn fmt::Write,
}

impl Renderer for Html<'_> {
    fn display_goal(&mut self, subtask_level: usize, goal: &Goal) -> fmt::Result {
        self.output.write_str(&format!(
            "{}{}\n",
            "  ".repeat(subtask_level),
            encode_text(&goal.to_string())
        ))
    }

    fn display_term(&mut self, subtask_level: usize, term: &TermProps) -> fmt::Result {
        self.output.write_str(&format!(
            "{}=> <b>{}</b>\n",
            "  ".repeat(subtask_level),
            encode_text(&term.term.to_string()),
        ))
    }

    fn display_reference(&mut self, subtask_level: usize, goal: &Goal) -> fmt::Result {
        writeln!(
            self.output,
            "<div class=\"goal ref\" style=\"margin-left:{}em\">{} <span class=\"ref-note\">(shown above)</span></div>",
            subtask_level * 2,
            encode_text(&goal.to_string())
        )
    }

    fn display_answer(
        &mut self,
        goal: &Goal,
        answer: Option<&Answer>,
        status: &Solution,
    ) -> fmt::Result {
        if let Some(answer) = answer.map(Answer::term) {
            self.output.write_str(&format!(
                "{} {}\n",
                match goal.kind() {
                    GoalKind::Find | GoalKind::Transform => {
                        format!("<b>Answer:</b> {}", encode_text(&answer.to_string()))
                    }
                    GoalKind::Prove => {
                        "<b>PROVEN!</b>".to_owned()
                    }
                },
                format_args!("[{} cycles, ]", status.cycles())
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

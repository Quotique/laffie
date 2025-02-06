use std::fmt;

use ratatui::{prelude::*, style::Stylize};

use solver::{
    task::{Purpose, Solver},
    term::{Term, TermProps},
};
use utils::VecDisplay;

use crate::Renderer;

#[derive(Default)]
pub struct Tui<'a> {
    pub output: Vec<Line<'a>>,
}

impl Renderer for Tui<'_> {
    fn display_purpose(&mut self, subtask_level: usize, purpose: &Purpose) -> fmt::Result {
        let purpose_style = Style::new().fg(Color::Blue).bold();
        self.output.push(Line::from(vec![Span::style(
            format!("{}{purpose}", "  ".repeat(subtask_level)).into(),
            purpose_style,
        )]));
        Ok(())
    }

    fn display_term(&mut self, subtask_level: usize, term: &TermProps) -> fmt::Result {
        let term_style = Style::new().fg(Color::Yellow).bold();
        self.output.push(Line::from(vec![
            Span::from(format!("{}=> ", "  ".repeat(subtask_level))),
            Span::style(term.term.to_string().into(), term_style),
            if term.requirements.is_empty() {
                Span::from("".to_owned())
            } else {
                Span::from(format!(" needed: [{}]", VecDisplay(&term.requirements)))
            },
        ]));
        Ok(())
    }

    fn display_answer(
        &mut self,
        purpose: &Purpose,
        answer: Option<&Term>,
        status: &Solver,
    ) -> fmt::Result {
        let answer_style = Style::new().fg(Color::Green).bold();
        let not_solved_style = Style::new().fg(Color::Red).bold().slow_blink();
        let cycles_counter_style = Style::new().fg(Color::Yellow);
        self.output
            .push(Line::from(if let Some(answer) = answer.as_ref() {
                vec![
                    Span::style(
                        match purpose {
                            Purpose::Find(_) | Purpose::Transform(_) => format!("Answer: {answer}"),
                            Purpose::Proof(_) => "PROOFED!".to_owned(),
                        }
                        .into(),
                        answer_style,
                    ),
                    Span::style(
                        format!(" [{} cycles]", status.current_cycles()).into(),
                        cycles_counter_style,
                    ),
                ]
            } else {
                vec![
                    Span::style("NOT SOLVED!".into(), not_solved_style),
                    Span::style(
                        format!(" [{} cycles]", status.current_cycles()).into(),
                        cycles_counter_style,
                    ),
                ]
            }));
        Ok(())
    }

    fn dump_frame(&mut self, frame: &[TermProps]) -> fmt::Result {
        self.output.append(
            &mut frame
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    Line::from(vec![Span::from(format!("{i} {} {:?}", s.term, s.parent))])
                })
                .collect(),
        );
        Ok(())
    }
}

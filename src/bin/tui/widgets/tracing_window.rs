use std::fmt::Display;

use itertools::{Itertools, chain};
use ratatui::{
    prelude::*,
    widgets::{List, Widget},
};

use solver::task::{Solution, SolutionStatus, TermInference};

use crate::{theme::Theme, widgets::tracing_navigation::TermId};

#[derive(Clone, Debug)]
pub struct TracingWindow {
    pub selected: Option<TermId>,
}

impl Widget for TracingWindow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if let Some(selected) = &self.selected {
            let text = if selected.idx == 0 {
                self.task_lines(selected.solution.as_ref())
            } else {
                self.term_inference_lines(selected)
            };
            <List as Widget>::render(
                List::new(text.iter().cloned()).highlight_style(self.theme().list_cursor_style()),
                area,
                buf,
            );
        }
    }
}

impl TracingWindow {
    fn task_lines<'a>(&'a self, solution: &Solution) -> Vec<Line<'a>> {
        vec![
            self.key_value_line("Task: ", &solution.goal),
            Line::default(),
            self.key_value_line(
                "Answer: ",
                solution
                    .answer()
                    .map(|x| Span::from(x.to_string()))
                    .unwrap_or(Span::styled("no answer", self.theme().error())),
            ),
            Line::default(),
            self.key_value_line("Cycles: ", solution.cycles()),
        ]
    }

    fn term_inference_lines(&self, term_id: &TermId) -> Vec<Line<'static>> {
        let term = &term_id.solution.terms[term_id.idx - 1];
        match &term.inference {
            TermInference::Rule {
                parent,
                params,
                rule,
                requirements,
            } => {
                let parent = &term_id.solution.terms[*parent];
                let mut result = vec![
                    self.key_value_line("Parent: ", parent),
                    self.key_value_line("Term: ", term),
                    Line::default(),
                    self.key_value_line("Rule: ", rule),
                    self.key_value_line("Params:", ""),
                ];
                result.append(
                    &mut chain(
                        params.params.iter().map(|(k, v)| self.params_line(k, v)),
                        { params.arglists.iter() }
                            .map(|(k, v)| self.params_line(k, v.iter().format(", "))),
                    )
                    .collect(),
                );
                result.push(Line::default());
                result.push(self.key_value_line("Requirements:", ""));

                let proven = self.theme().proven_requirement();
                let unproven = self.theme().unproven_requirement();
                let skipped = self.theme().skipped_requirement();
                for i in requirements {
                    let goal = &i.task.goal.term;
                    result.push(Line::from(match i.status {
                        SolutionStatus::Answer(_) => Span::styled(format!("  ☑  {goal}"), proven),
                        SolutionStatus::Err(_) => Span::styled(format!("  ☒  {goal}"), unproven),
                        SolutionStatus::NotDone => Span::styled(format!("  ☐  {goal}"), skipped),
                    }));
                }
                result.push(Line::default());
                result.push(self.key_value_line("Cycles: ", 0));
                result
            }

            _ => {
                vec![self.key_value_line("Term: ", term)]
            }
        }
    }

    fn key_value_line<'a>(&self, key: &'a str, text: impl Display) -> Line<'a> {
        Line::from(vec![
            Span::styled(key, self.theme().highlighted()),
            Span::from(text.to_string()),
        ])
    }

    fn params_line(&self, key: impl Display, text: impl Display) -> Line<'static> {
        Line::from(vec![Span::raw(format!("  {key} = {text}"))])
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

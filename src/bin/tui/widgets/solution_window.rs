use std::fmt::Display;

use ratatui::{
    prelude::*,
    widgets::{List, StatefulWidget},
};
use trees::Tree;

use solver::task::{SharedSolution, SolutionStatus, StepsSource, Task, Visit};
use utils::TreeIndex;

use crate::{
    state::{DirectoryStat, TasksNode},
    theme::{draw_scrollbar_buf, Theme},
};

pub struct SolutionWindow<'a> {
    pub tasks_index: &'a mut Tree<TasksNode>,

    pub selected: Option<TreeIndex>,
}

impl<'a> StatefulWidget for SolutionWindow<'a> {
    type State = ();

    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        let Some(selected) = &self.selected else {
            return;
        };
        let list_cursor = self.theme().list_cursor_style();
        let list = match self.tasks_index[selected].data() {
            TasksNode::Task(tracing) => {
                let mut lines = self.task_lines(&tracing.solution.task);
                if tracing.solution.status != SolutionStatus::NotDone {
                    lines.extend(self.solution_status_lines(tracing.solution.clone()));
                } else {
                    lines.push(Line::from(Span::from("Press s to solve".to_owned())));
                };
                List::new(lines.iter().cloned()).highlight_style(list_cursor)
            }
            TasksNode::Directory(dir) => List::new(self.dir_status_lines(dir))
                .highlight_style(self.theme().list_cursor_style()),
        };
        match self.tasks_index[selected].data_mut() {
            TasksNode::Task(tracing) => {
                let scroll_pos = tracing.solution_pos.selected().unwrap();
                let list_len = list.len();
                <List as StatefulWidget>::render(list, area, buf, &mut tracing.solution_pos);
                draw_scrollbar_buf(buf, area, list_len, scroll_pos);
            }
            TasksNode::Directory(_) => {
                <List as Widget>::render(list, area, buf);
            }
        }
    }
}

impl<'a> SolutionWindow<'a> {
    fn task_lines(&self, task: &Task) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(vec![
                Span::styled(format!("{:x}", task.id), self.theme().highlighted()),
                Span::from(". "),
                Span::from(task.text.clone()),
            ]),
            Line::default(),
            Line::from(Span::styled(
                task.purpose.to_string(),
                self.theme().solution_purpose(),
            )),
        ];
        for condition in &task.conditions {
            lines.push(Line::from(Span::styled(
                format!("  {condition}"),
                self.theme().solution_term(),
            )));
        }
        lines.push(Line::default());
        lines
    }

    fn solution_status_lines(&self, solution: SharedSolution) -> Vec<Line<'static>> {
        let mut lines = vec![];
        let mut depth = 0;

        for step in solution.steps() {
            match step {
                Visit::Subtask(t) => {
                    lines.push(Line::from(Span::styled(
                        format!("{}{}", "  ".repeat(depth), t.purpose),
                        self.theme().solution_purpose(),
                    )));
                    depth += 1;
                }
                Visit::Term(t) => {
                    lines.push(Line::from(Span::styled(
                        format!("{}{t}", "  ".repeat(depth)),
                        self.theme().solution_term(),
                    )));
                }
                Visit::Answer(a) => {
                    lines.push(Line::from(Span::styled(
                        format!("{}{a}", "  ".repeat(depth)),
                        self.theme().solution_answer(),
                    )));
                    depth -= 1;
                }
            }
        }
        lines
    }

    fn dir_status_lines(&self, dir: &DirectoryStat) -> impl Iterator<Item = Line<'static>> {
        let wrong_answer = self.theme().wrong_answer();
        let unsolved = self.theme().unsolved();
        let solved = self.theme().solved();
        let not_started = self.theme().not_started();
        let default = self.theme().default();

        [
            self.pair_line("Group: ", &dir.dir_name, default),
            Line::default(),
            self.pair_line("Total: ", dir.total(), default),
            Line::default(),
            self.pair_line("Not started: ", dir.not_started_count, not_started),
            self.pair_line("Solved: ", dir.solved_count, solved),
            self.pair_line("Not solved: ", dir.unsolved_count, unsolved),
            self.pair_line("Wrong answers: ", dir.wrong_answer_count, wrong_answer),
        ]
        .into_iter()
    }

    fn pair_line<'b>(&self, k: &'b str, v: impl Display, v_style: Style) -> Line<'b> {
        let highlighted = self.theme().highlighted();
        Line::from(vec![
            Span::styled(k, highlighted),
            Span::styled(v.to_string(), v_style),
        ])
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

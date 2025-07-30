use std::fmt::Display;

use ratatui::{
    prelude::*,
    widgets::{List, StatefulWidget},
};
use trees::Tree;

use solver::task::{SolutionStatus, StepsSource};
use utils::TreeIndex;

use crate::{
    tasks::TaskState,
    theme::{draw_scrollbar_buf, Theme},
    widgets::tasks_list::{DirectoryStatus, TasksNode},
};

pub struct SolutionWindow<'a> {
    pub tasks_index: &'a Tree<TasksNode>,
    pub tasks:       &'a mut [TaskState],

    pub selected: Option<TreeIndex>,
}

impl<'a> StatefulWidget for SolutionWindow<'a> {
    type State = ();

    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        let Some(selected) = &self.selected else {
            return;
        };
        match self.tasks_index[selected].data() {
            TasksNode::Task(task_id) => {
                let tracing = &self.tasks[*task_id];
                let mut lines: Vec<_> = format!("Task {}\n\nSolution", tracing.solution.task)
                    .split('\n')
                    .map(|x| Line::from(Span::from(x.to_owned())))
                    .collect();

                if tracing.solution.status != SolutionStatus::NotDone {
                    lines.extend(
                        // TODO: format
                        { tracing.solution.steps() }.map(|x| {
                            Line::from(Span::styled(x.to_string(), self.theme().default()))
                        }),
                    );
                } else {
                    lines.push(Line::from(Span::from("Press s to solve".to_owned())));
                };
                let scroll_pos = tracing.solution_pos.selected().unwrap();
                <List as StatefulWidget>::render(
                    List::new(lines.iter().cloned())
                        .highlight_style(self.theme().list_cursor_style()),
                    area,
                    buf,
                    &mut self.tasks[*task_id].solution_pos,
                );
                draw_scrollbar_buf(buf, area, lines.len(), scroll_pos);
            }
            TasksNode::Directory(dir) => {
                <List as Widget>::render(
                    List::new(self.dir_status_lines(dir))
                        .highlight_style(self.theme().list_cursor_style()),
                    area,
                    buf,
                );
            }
        };
    }
}

impl<'a> SolutionWindow<'a> {
    fn dir_status_lines(&self, dir: &DirectoryStatus) -> impl Iterator<Item = Line<'static>> {
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

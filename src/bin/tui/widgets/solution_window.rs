use std::fmt::Display;

use ratatui::{
    prelude::*,
    widgets::{List, StatefulWidget},
};
use trees::Tree;
use tui_scrollview::{ScrollView, ScrollViewState, ScrollbarVisibility};

use solver::task::{SharedSolution, SolutionStatus, StepsSource, Task, Visit};
use utils::TreeIndex;

use crate::{
    state::{DirectoryStat, ProblemTask, TaskStatusKind, TasksNode, collect_problem_tasks},
    theme::Theme,
};

pub struct SolutionWindow<'a> {
    pub tasks_index: &'a mut Tree<TasksNode>,
    pub dir_scroll:  &'a mut ScrollViewState,

    pub selected: Option<TreeIndex>,
}

impl<'a> StatefulWidget for SolutionWindow<'a> {
    type State = ();

    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        let Some(selected) = &self.selected else {
            return;
        };
        let list_cursor = self.theme().list_cursor_style();
        let (lines, is_directory) = match self.tasks_index[selected].data() {
            TasksNode::Task(tracing) => {
                let mut lines = self.task_lines(&tracing.solution.task);
                if tracing.solution.status != SolutionStatus::NotDone {
                    lines.extend(self.solution_status_lines(tracing.solution.clone()));
                } else {
                    lines.push(Line::from(Span::from("Press s to solve".to_owned())));
                };
                (lines, false)
            }
            TasksNode::Directory(dir) => {
                let mut problems = Vec::new();
                collect_problem_tasks(&self.tasks_index[selected], &mut problems);
                (self.dir_lines(dir, &problems), true)
            }
        };
        let list = List::new(lines.iter().cloned()).highlight_style(list_cursor);
        let height = lines.len() as u16;

        if is_directory {
            let content_size = Size::new(area.as_size().width, height);
            let mut view = ScrollView::new(content_size)
                .horizontal_scrollbar_visibility(ScrollbarVisibility::Never)
                .vertical_scrollbar_visibility(ScrollbarVisibility::Automatic);
            <List as Widget>::render(list, view.buf().area, view.buf_mut());
            view.render(area, buf, self.dir_scroll);
        } else if let TasksNode::Task(tracing) = self.tasks_index[selected].data_mut() {
            let content_size = Size::new(area.as_size().width, height);
            let mut view = ScrollView::new(content_size)
                .horizontal_scrollbar_visibility(ScrollbarVisibility::Never)
                .vertical_scrollbar_visibility(ScrollbarVisibility::Automatic);
            <List as Widget>::render(list, view.buf().area, view.buf_mut());
            view.render(area, buf, &mut tracing.solution_pos);
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
                task.goal.to_string(),
                self.theme().solution_goal(),
            )),
        ];
        for condition in &task.givens {
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
                        format!("{}{}", "  ".repeat(depth), t.goal),
                        self.theme().solution_goal(),
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

    fn dir_lines(&self, dir: &DirectoryStat, problems: &[ProblemTask]) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = self.dir_status_lines(dir).collect();
        if !problems.is_empty() {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                format!("Problem tasks ({}):", problems.len()),
                self.theme().highlighted(),
            )));
            for pt in problems {
                let (badge, badge_style) = match pt.kind {
                    TaskStatusKind::WrongAnswer => ("[wrong]   ", self.theme().wrong_answer()),
                    TaskStatusKind::Unsolved => ("[unsolved]", self.theme().unsolved()),
                    TaskStatusKind::Solved => ("[solved]  ", self.theme().solved()),
                    TaskStatusKind::NotStarted => ("[idle]    ", self.theme().not_started()),
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {badge}"), badge_style),
                    Span::from(" "),
                    Span::styled(format!("{:x}", pt.task_id), self.theme().highlighted()),
                    Span::from(". "),
                    Span::raw(pt.text.clone()),
                ]));
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

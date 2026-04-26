use std::{collections::HashMap, fmt::Display};

use ratatui::{
    prelude::*,
    widgets::{List, StatefulWidget},
};
use trees::Tree;
use tui_scrollview::{ScrollView, ScrollViewState, ScrollbarVisibility};

use solver::task::{SharedSolution, SolutionStatus, StepsSource, Task, TermInference, Visit};
use utils::TreeIndex;

use crate::{
    state::{DirectoryStat, ProblemTask, TaskStatusKind, TasksNode, collect_problem_tasks},
    strings,
    theme::Theme,
};

pub struct SolutionWindow<'a> {
    pub tasks_index: &'a mut Tree<TasksNode>,
    pub dir_scroll:  &'a mut ScrollViewState,
    pub theme:       &'a Theme,

    pub selected: Option<TreeIndex>,
}

impl<'a> StatefulWidget for SolutionWindow<'a> {
    type State = ();

    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        let Some(selected) = &self.selected else {
            return;
        };
        let list_cursor = self.theme.list_cursor;
        let (lines, is_directory) = match self.tasks_index[selected].data() {
            TasksNode::Task(tracing) => {
                let mut lines = self.task_lines(&tracing.solution.task);
                if tracing.solution.status != SolutionStatus::NotDone {
                    lines.extend(self.solution_status_lines(tracing.solution.clone()));
                    if let Some(prev) = &tracing.previous_solution {
                        lines.extend(self.diff_lines(prev, &tracing.solution));
                    }
                } else {
                    lines.push(Line::from(Span::from(strings::solution::PRESS_S_TO_SOLVE)));
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
                Span::styled(format!("{:x}", task.id), self.theme.highlighted),
                Span::from(". "),
                Span::from(task.text.clone()),
            ]),
            Line::default(),
            Line::from(Span::styled(
                task.goal.to_string(),
                self.theme.solution_goal,
            )),
        ];
        for condition in &task.givens {
            lines.push(Line::from(Span::styled(
                format!("  {condition}"),
                self.theme.solution_term,
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
                        self.theme.solution_goal,
                    )));
                    depth += 1;
                }
                Visit::Term(t) => {
                    lines.push(Line::from(Span::styled(
                        format!("{}{t}", "  ".repeat(depth)),
                        self.theme.solution_term,
                    )));
                }
                Visit::Answer(a) => {
                    lines.push(Line::from(Span::styled(
                        format!("{}{a}", "  ".repeat(depth)),
                        self.theme.solution_answer,
                    )));
                    depth -= 1;
                }
            }
        }
        lines
    }

    fn diff_lines(
        &self,
        previous: &SharedSolution,
        current: &SharedSolution,
    ) -> Vec<Line<'static>> {
        let prev_steps = step_signatures(previous);
        let cur_steps = step_signatures(current);

        let mut counts: HashMap<&String, (i32, i32)> = HashMap::new();
        for s in &prev_steps {
            counts.entry(s).or_default().0 += 1;
        }
        for s in &cur_steps {
            counts.entry(s).or_default().1 += 1;
        }

        let mut added: Vec<(&String, i32)> = Vec::new();
        let mut removed: Vec<(&String, i32)> = Vec::new();
        for (sig, (p, c)) in &counts {
            match c.cmp(p) {
                std::cmp::Ordering::Greater => added.push((sig, c - p)),
                std::cmp::Ordering::Less => removed.push((sig, p - c)),
                std::cmp::Ordering::Equal => {}
            }
        }
        added.sort_by(|a, b| a.0.cmp(b.0));
        removed.sort_by(|a, b| a.0.cmp(b.0));

        let mut lines = vec![
            Line::default(),
            Line::from(Span::styled(
                strings::solution::diff_header(prev_steps.len(), cur_steps.len()),
                self.theme.highlighted,
            )),
        ];
        if added.is_empty() && removed.is_empty() {
            lines.push(Line::from(Span::raw(strings::solution::DIFF_IDENTICAL)));
            return lines;
        }
        for (sig, n) in &added {
            lines.push(Line::from(Span::styled(
                diff_line('+', *n, sig),
                self.theme.solved,
            )));
        }
        for (sig, n) in &removed {
            lines.push(Line::from(Span::styled(
                diff_line('-', *n, sig),
                self.theme.wrong_answer,
            )));
        }
        lines
    }

    fn dir_lines(&self, dir: &DirectoryStat, problems: &[ProblemTask]) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = self.dir_status_lines(dir).collect();
        if !problems.is_empty() {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                strings::solution::problem_tasks_header(problems.len()),
                self.theme.highlighted,
            )));
            for pt in problems {
                let (badge, badge_style) = match pt.kind {
                    TaskStatusKind::WrongAnswer => {
                        (strings::status_badge::WRONG, self.theme.wrong_answer)
                    }
                    TaskStatusKind::Unsolved => {
                        (strings::status_badge::UNSOLVED, self.theme.unsolved)
                    }
                    TaskStatusKind::Solved => (strings::status_badge::SOLVED, self.theme.solved),
                    TaskStatusKind::NotStarted => {
                        (strings::status_badge::NOT_STARTED, self.theme.not_started)
                    }
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {badge}"), badge_style),
                    Span::from(" "),
                    Span::styled(format!("{:x}", pt.task_id), self.theme.highlighted),
                    Span::from(". "),
                    Span::raw(pt.text.clone()),
                ]));
            }
        }
        lines
    }

    fn dir_status_lines(&self, dir: &DirectoryStat) -> impl Iterator<Item = Line<'static>> {
        let wrong_answer = self.theme.wrong_answer;
        let unsolved = self.theme.unsolved;
        let solved = self.theme.solved;
        let not_started = self.theme.not_started;
        let default = self.theme.default;

        [
            self.pair_line(strings::directory_summary::GROUP, &dir.dir_name, default),
            Line::default(),
            self.pair_line(strings::directory_summary::TOTAL, dir.total(), default),
            Line::default(),
            self.pair_line(
                strings::directory_summary::NOT_STARTED,
                dir.not_started_count,
                not_started,
            ),
            self.pair_line(strings::directory_summary::SOLVED, dir.solved_count, solved),
            self.pair_line(
                strings::directory_summary::NOT_SOLVED,
                dir.unsolved_count,
                unsolved,
            ),
            self.pair_line(
                strings::directory_summary::WRONG_ANSWERS,
                dir.wrong_answer_count,
                wrong_answer,
            ),
        ]
        .into_iter()
    }

    fn pair_line<'b>(&self, k: &'b str, v: impl Display, v_style: Style) -> Line<'b> {
        Line::from(vec![
            Span::styled(k, self.theme.highlighted),
            Span::styled(v.to_string(), v_style),
        ])
    }
}

fn diff_line(sign: char, count: i32, sig: &str) -> String {
    if count > 1 {
        format!("  {sign}{count}× {sig}")
    } else {
        format!("  {sign} {sig}")
    }
}

fn step_signatures(solution: &SharedSolution) -> Vec<String> {
    let answer_path = match solution.status {
        SolutionStatus::Answer(answer_idx) => {
            let mut q = vec![answer_idx];
            while let Some(p) = solution.terms[*q.last().unwrap()].inference.parent_id() {
                q.push(p);
            }
            Some(q)
        }
        _ => None,
    };

    let indices: Vec<usize> = match answer_path {
        Some(q) => q.into_iter().rev().collect(),
        None => solution
            .terms
            .iter()
            .enumerate()
            .filter(|(_, t)| t.inference.is_proven())
            .map(|(n, _)| n)
            .collect(),
    };

    indices
        .into_iter()
        .map(|i| {
            let t = &solution.terms[i];
            let label = match &t.inference {
                TermInference::Rule { rule, .. } => format!("← {}", rule.id),
                TermInference::Transform { .. } => "← transform".to_string(),
                TermInference::Condition => "(given)".to_string(),
            };
            format!("{}  {label}", t.term)
        })
        .collect()
}

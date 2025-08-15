use std::sync::Arc;

use itertools::Itertools;
use parking_lot::Mutex;
use ratatui::{prelude::*, widgets::Paragraph};

use solver::task::{Task, TermProps, Tracer};

use super::popup::Popup;
use crate::theme::Theme;

#[derive(Clone)]
pub struct ProgressReporter(pub Arc<Mutex<SolverProgress>>);

#[derive(Clone, Debug, Default)]
pub struct SolverProgress {
    pub current_task: Option<Task>,

    pub current_cycles:       usize,
    pub exec_deadline:        usize,
    pub finished_tasks_count: usize,
    pub total_tasks_count:    usize,
}

impl Tracer for ProgressReporter {
    fn on_term_focus(&mut self, _term: &TermProps, cycle: usize) {
        self.0.lock().current_cycles = cycle;
    }
}

impl SolverProgress {
    pub fn new(exec_deadline: usize) -> Self {
        SolverProgress {
            exec_deadline,
            ..Default::default()
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let mut popup = Popup::new(Line::from("Solving in progress").centered());
        popup.draw(frame, area);
        let inner = popup.inner(area);
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(100),
                Constraint::Min(3),
                Constraint::Min(3),
                Constraint::Min(3),
            ])
            .split(inner);

        let task_text = if let Some(task) = self.current_task.as_ref() {
            vec![
                Line::from(Span::styled(
                    task.purpose.to_string(),
                    self.theme().highlighted(),
                )),
                Line::default(),
                Line::from(Span::styled(
                    task.conditions.iter().format(", ").to_string(),
                    self.theme().solution_term(),
                )),
            ]
        } else {
            vec![]
        };

        Paragraph::new(task_text).render(layout[0], frame.buffer_mut());
        let current_percent = ((self.current_cycles * 100) / self.exec_deadline) as u16;
        let total_percent =
            (((self.finished_tasks_count * self.exec_deadline + self.current_cycles) * 100) /
                (self.total_tasks_count * self.exec_deadline)) as u16;

        self.theme()
            .gauge(Line::from("Current").left_aligned())
            .percent(current_percent)
            .render(layout[1], frame.buffer_mut());
        self.theme()
            .gauge(Line::from("Total").left_aligned())
            .percent(total_percent)
            .render(layout[2], frame.buffer_mut());
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

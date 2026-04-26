use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use itertools::Itertools;
use ratatui::{prelude::*, widgets::Paragraph};

use solver::task::{Task, TermProps, Tracer};

use super::popup::Popup;
use crate::theme::Theme;

pub enum ProgressEvent {
    TaskStarted(Box<Task>),
    TaskFinished,
}

#[derive(Clone)]
pub struct ProgressReporter {
    pub cancel: Arc<AtomicBool>,
    pub cycles: Arc<AtomicUsize>,
}

#[derive(Debug)]
pub struct SolverProgress {
    pub current_task:         Option<Task>,
    pub exec_deadline:        usize,
    pub finished_tasks_count: usize,
    pub total_tasks_count:    usize,
}

impl Tracer for ProgressReporter {
    fn on_term_focus(&mut self, _term: &TermProps, cycle: usize) {
        self.cycles.store(cycle, Ordering::Relaxed);
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

impl SolverProgress {
    pub fn new(exec_deadline: usize) -> Self {
        Self {
            current_task: None,
            exec_deadline,
            finished_tasks_count: 0,
            total_tasks_count: 0,
        }
    }

    pub fn reset(&mut self, total_tasks_count: usize) {
        self.current_task = None;
        self.finished_tasks_count = 0;
        self.total_tasks_count = total_tasks_count;
    }

    pub fn handle(&mut self, event: ProgressEvent) {
        match event {
            ProgressEvent::TaskStarted(task) => self.current_task = Some(*task),
            ProgressEvent::TaskFinished => self.finished_tasks_count += 1,
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, current_cycles: usize, queued: usize) {
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
                    task.goal.to_string(),
                    self.theme().highlighted(),
                )),
                Line::default(),
                Line::from(Span::styled(
                    task.givens.iter().format(", ").to_string(),
                    self.theme().solution_term(),
                )),
            ]
        } else {
            vec![]
        };

        Paragraph::new(task_text).render(layout[0], frame.buffer_mut());
        let deadline = self.exec_deadline.max(1);
        let total = self.total_tasks_count.max(1);
        let current_percent = ((current_cycles * 100) / deadline).min(100) as u16;
        let total_percent = (((self.finished_tasks_count * deadline + current_cycles) * 100) /
            (total * deadline))
            .min(100) as u16;

        let total_label = if queued > 0 {
            format!(
                "Total {}/{} (+{queued} queued)",
                self.finished_tasks_count, self.total_tasks_count
            )
        } else {
            format!(
                "Total {}/{}",
                self.finished_tasks_count, self.total_tasks_count
            )
        };

        self.theme()
            .gauge(Line::from("Current").left_aligned())
            .percent(current_percent)
            .render(layout[1], frame.buffer_mut());
        self.theme()
            .gauge(Line::from(total_label).left_aligned())
            .percent(total_percent)
            .render(layout[2], frame.buffer_mut());

        let cancel_block = self.theme().block(true, "");
        let inner = cancel_block.inner(layout[3]);
        cancel_block.render(layout[3], frame.buffer_mut());

        Line::from("Press C to Cancel")
            .centered()
            .render(inner, frame.buffer_mut());
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

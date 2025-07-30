use ratatui::prelude::*;
use tui_tree_widget::TreeState;

use super::state::Command;
use crate::{
    tasks::TaskStatus,
    theme::Theme,
    widgets::{
        tracing_tree::{TermId, TracingTree},
        tracing_window::TracingWindow,
    },
};

pub struct Tracing {
    pub task: TaskStatus,

    tree_state: TreeState<TermId>,
}

impl Tracing {
    pub fn new(task: TaskStatus) -> Self {
        Self {
            task,
            tree_state: Default::default(),
        }
    }

    pub fn process(&mut self, command: Command) {
        let _ = match command {
            Command::Down => self.tree_state.key_down(),
            Command::Up => self.tree_state.key_up(),
            Command::Left => self.tree_state.key_left(),
            Command::Right => self.tree_state.key_right(),
            Command::Toggle => self.tree_state.toggle_selected(),
            _ => false,
        };
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let block = self.theme().block(true, "Tracing");
        let inner = block.inner(layout[0]);
        frame.render_widget(block, layout[0]);
        frame.render_stateful_widget(
            TracingTree {
                solution: self.task.solution.clone(),
            },
            inner,
            &mut self.tree_state,
        );

        let block = self.theme().block(false, "Detailed");
        let inner = block.inner(layout[1]);
        frame.render_widget(block, layout[1]);
        frame.render_widget(
            TracingWindow {
                selected: self.tree_state.selected().last().cloned(),
            },
            inner,
        );
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

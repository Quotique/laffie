use ratatui::prelude::*;
use tui_tree_widget::TreeState;

use super::state::Command;
use crate::{
    tasks::TaskStatus,
    widgets::{
        tracing_tree::{TermId, TracingTree},
        tracing_window::TracingWindow,
    },
};

pub struct Tracing {
    pub task: TaskStatus,

    tree_state:   TreeState<TermId>,
    focused_pane: usize,
}

impl Tracing {
    pub fn new(task: TaskStatus) -> Self {
        Self {
            task,
            tree_state: Default::default(),
            focused_pane: 0,
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

        frame.render_stateful_widget(
            TracingTree {
                solution: self.task.solution.clone(),
            },
            layout[0],
            &mut self.tree_state,
        );
        frame.render_widget(
            TracingWindow {
                selected:   self.tree_state.selected().last().cloned(),
                is_focused: self.focused_pane == 1,
            },
            layout[1],
        );
    }
}

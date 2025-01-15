use std::{path::Path, sync::Arc};

use derive_more::Display;
use ratatui::{
    prelude::*,
    widgets::{ListState, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use parser::DirectoryParser;

use super::{rules::Rules, tasks::Tasks};

pub struct Status {
    pub current_tab: Tab,

    rules: Rules,
    tasks: Tasks,
}

#[derive(Debug, Clone, Copy, Display, Eq, PartialEq)]
pub enum Tab {
    Rules,
    Tasks,
    Tracing,
    // Setting,
}

impl Status {
    pub fn new(symbols_dir: impl AsRef<Path>, tasks_dir: impl AsRef<Path>) -> Self {
        let parser = DirectoryParser::new(symbols_dir.as_ref(), tasks_dir.as_ref());

        let rules = Arc::new(parser.load_rules().unwrap());
        let tasks = parser.load_tasks().unwrap();

        Status {
            current_tab: Tab::Rules,
            tasks:       Tasks::new(rules.clone(), tasks),

            rules: Rules::new(rules),
        }
    }

    pub fn solve(&mut self) {
        if self.current_tab == Tab::Tasks {
            self.tasks.solve();
        }
    }

    pub fn next(&mut self) {
        match self.current_tab {
            Tab::Rules => self.rules.select_next(),
            Tab::Tasks => self.tasks.select_next(),
            Tab::Tracing => self.tasks.tracing().select_next(),
        }
    }

    pub fn previous(&mut self) {
        match self.current_tab {
            Tab::Rules => self.rules.select_previous(),
            Tab::Tasks => self.tasks.select_previous(),
            Tab::Tracing => self.tasks.tracing().select_previous(),
        }
    }

    pub fn left(&mut self) {
        match self.current_tab {
            Tab::Rules => self.rules.left(),
            Tab::Tasks => self.tasks.left(),
            Tab::Tracing => self.tasks.tracing().left(),
        }
    }

    pub fn right(&mut self) {
        match self.current_tab {
            Tab::Rules => self.rules.right(),
            Tab::Tasks => self.tasks.right(),
            Tab::Tracing => self.tasks.tracing().right(),
        }
    }

    pub fn toggle(&mut self) {
        if self.current_tab == Tab::Tracing {
            self.tasks.tracing().toggle()
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        match self.current_tab {
            Tab::Rules => self.rules.draw(frame, area),
            Tab::Tasks => self.tasks.draw(frame, area),
            Tab::Tracing => self.tasks.tracing().draw(frame, area),
        }
    }
}

impl Tab {
    pub const MAX: usize = 2;
}

impl From<Tab> for usize {
    fn from(val: Tab) -> Self {
        match val {
            Tab::Rules => 0,
            Tab::Tasks => 1,
            Tab::Tracing => 2,
            // Tab::Setting => 3,
        }
    }
}

impl From<usize> for Tab {
    fn from(value: usize) -> Self {
        match value {
            0 => Tab::Rules,
            1 => Tab::Tasks,
            2 => Tab::Tracing,
            _ => unimplemented!(),
            // _ => Tab::Setting,
        }
    }
}

pub fn border_focus() -> Style {
    Style::new().fg(Color::Cyan)
}

pub fn border_unfocus() -> Style {
    Style::new()
}

pub fn draw_scrollbar(frame: &mut Frame, area: Rect, len: usize, pos: usize) {
    let mut scrollbar_state = ScrollbarState::new(len).position(pos);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        area.inner(Margin {
            vertical:   1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
}

pub fn default_state() -> ListState {
    let mut state = ListState::default();
    state.select(Some(0));
    state
}

use std::sync::Arc;

use ratatui::{
    prelude::*,
    widgets::{List, ListState, Paragraph},
};

use solver::rule::RulesEngine;

use super::{
    state::{default_state, Command},
    theme::{draw_scrollbar, Theme},
};

pub struct Rules {
    engine:       Arc<RulesEngine>,
    list_state:   ListState,
    focused_pane: usize,
}

impl Rules {
    #[inline]
    pub fn new(engine: Arc<RulesEngine>) -> Self {
        Self {
            engine,
            list_state: default_state(),
            focused_pane: 0,
        }
    }

    pub fn process(&mut self, command: Command) {
        match command {
            Command::Down => self.list_state.select_next(),
            Command::Up => self.list_state.select_previous(),
            Command::Left => self.focused_pane = 0,
            Command::Right => self.focused_pane = 1,
            _ => {}
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let items: Vec<_> = self.engine.iter().collect();
        let list = List::new(items.iter().map(|x| x.term.to_string()))
            .block(self.theme().block(self.focused_pane == 0, "Rules"))
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, layout[0], &mut self.list_state);
        draw_scrollbar(
            frame,
            layout[0],
            items.len(),
            self.list_state.selected().unwrap(),
        );

        frame.render_widget(
            Paragraph::new(
                items
                    .get(self.list_state.selected().unwrap())
                    .unwrap()
                    .to_string(),
            )
            .block(self.theme().block(self.focused_pane == 1, "Detailed")),
            layout[1],
        );
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

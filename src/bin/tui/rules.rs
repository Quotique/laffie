use std::sync::Arc;

use ratatui::{prelude::*, widgets::ListState};

use solver::rule::RulesEngine;

use super::state::{default_state, Command};
use crate::{
    theme::Theme,
    widgets::{rule_window::RuleWindow, rules_list::RulesList},
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

        let rulse_list = RulesList {
            engine: self.engine.clone(),
        };
        let block = self.theme().block(self.focused_pane == 0, "Rules");
        let inner = block.inner(layout[0]);
        frame.render_widget(block, layout[0]);
        frame.render_stateful_widget(rulse_list, inner, &mut self.list_state);

        let rule_window = RuleWindow {
            rule: { self.engine.iter() }
                .nth(self.list_state.selected().expect("missing selected"))
                .expect("rule not found"),
        };
        let block = self.theme().block(self.focused_pane == 1, "Detailed");
        let inner = block.inner(layout[1]);
        frame.render_widget(block, layout[1]);
        frame.render_widget(rule_window, inner);
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

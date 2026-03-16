use std::sync::Arc;

use ratatui::{
    prelude::*,
    widgets::{List, ListState, StatefulWidget},
};

use solver::rule::RulesEngine;

use crate::theme::{Theme, draw_scrollbar_buf};

pub struct RulesList {
    pub engine: Arc<RulesEngine>,
}

impl StatefulWidget for RulesList {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let list = List::new(self.engine.iter().map(|x| format!("{}. {}", x.id, x.term)))
            .highlight_symbol("> ");

        let len = list.len();
        <List as StatefulWidget>::render(list, area, buf, state);
        draw_scrollbar_buf(buf, area, len, state.selected().unwrap());
    }
}

impl RulesList {
    fn _theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

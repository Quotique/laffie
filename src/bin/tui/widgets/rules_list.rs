use ratatui::{
    prelude::*,
    widgets::{List, ListState, StatefulWidget},
};

use solver::rule::SharedRule;

use crate::theme::draw_scrollbar_buf;

pub struct RulesList {
    pub items: Vec<SharedRule>,
}

impl StatefulWidget for RulesList {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let list = List::new(self.items.iter().map(|x| format!("{}. {}", x.id, x.term)))
            .highlight_symbol("> ");

        let len = list.len();
        <List as StatefulWidget>::render(list, area, buf, state);
        draw_scrollbar_buf(buf, area, len, state.selected().unwrap_or(0));
    }
}

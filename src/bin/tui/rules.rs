use std::sync::Arc;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListState, Paragraph},
};

use mcore::rule::RulesEngine;

use super::interface::{border_focus, border_unfocus, default_state, draw_scrollbar};

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

    #[inline]
    pub fn select_next(&mut self) {
        self.list_state.select_next()
    }

    #[inline]
    pub fn select_previous(&mut self) {
        self.list_state.select_previous()
    }

    #[inline]
    pub fn left(&mut self) {
        self.focused_pane = 0;
    }

    #[inline]
    pub fn right(&mut self) {
        self.focused_pane = 1;
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let items: Vec<_> = self.engine.iter().collect();
        let list = List::new(items.iter().map(|x| x.term.to_string()))
            .highlight_symbol("> ")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.pane_style(0))
                    .title("Rules"),
            );

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
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.pane_style(1))
                    .title("Detailed"),
            ),
            layout[1],
        );
    }

    fn pane_style(&self, pane: usize) -> Style {
        if self.focused_pane == pane {
            border_focus()
        } else {
            border_unfocus()
        }
    }
}

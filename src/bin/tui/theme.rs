use ratatui::{
    prelude::*,
    style::{Style, Stylize},
    widgets::{
        block::Title, Block, Borders, Gauge, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

pub struct Theme {}

impl Theme {
    pub fn block<'a>(&self, focused: bool, title: impl Into<Title<'a>>) -> Block<'a> {
        let pane_style = if focused {
            Style::new().fg(Color::Cyan)
        } else {
            Style::new()
        };
        Block::default()
            .borders(Borders::ALL)
            .border_style(pane_style)
            .title(title)
    }

    pub fn gauge<'a>(&self, title: impl Into<Title<'a>>) -> Gauge<'a> {
        let title = self.block(false, title);
        Gauge::default()
            .block(title)
            .gauge_style(Style::default().fg(Color::Blue).bg(Color::DarkGray))
    }

    pub fn scrollbar(&self) -> Scrollbar<'static> {
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None)
    }

    pub fn tree_cursor_style(&self) -> Style {
        Style::new()
            .fg(Color::Black)
            .bg(Color::LightGreen)
            .add_modifier(Modifier::BOLD)
    }

    pub fn list_cursor_style(&self) -> Style {
        Style::new().underlined()
    }

    pub fn wrong_answer(&self) -> Style {
        Style::new().fg(Color::Red)
    }

    pub fn unsolved(&self) -> Style {
        Style::new().fg(Color::Yellow)
    }

    pub fn solved(&self) -> Style {
        Style::new().fg(Color::Green)
    }

    pub fn not_started(&self) -> Style {
        Style::new()
    }

    pub fn default(&self) -> Style {
        Style::new()
    }

    pub fn highlighted(&self) -> Style {
        Style::new().fg(Color::LightBlue).bold()
    }

    pub fn error(&self) -> Style {
        Style::new().fg(Color::Red).bold()
    }

    pub fn default_tree_item(&self) -> Style {
        Style::new()
    }

    pub fn unproven_tree_item(&self) -> Style {
        Style::new().crossed_out().dim()
    }

    pub fn proven_requirement(&self) -> Style {
        Style::new().fg(Color::Green).bold()
    }

    pub fn unproven_requirement(&self) -> Style {
        Style::new().fg(Color::Red).bold()
    }

    pub fn skipped_requirement(&self) -> Style {
        Style::new().fg(Color::Gray).bold()
    }

    pub fn solution_answer(&self) -> Style {
        Style::new().fg(Color::Green).italic()
    }

    pub fn solution_term(&self) -> Style {
        Style::new().fg(Color::Yellow)
    }

    pub fn solution_goal(&self) -> Style {
        Style::new().fg(Color::Cyan).bold()
    }
}

pub fn draw_scrollbar_buf(buf: &mut Buffer, area: Rect, len: usize, pos: usize) {
    let mut scrollbar_state = ScrollbarState::new(len).position(pos);
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .render(
            area.inner(Margin {
                vertical:   1,
                horizontal: 0,
            }),
            buf,
            &mut scrollbar_state,
        );
}

use ratatui::{
    prelude::*,
    style::{Style, Stylize},
    widgets::{
        Block, Borders, Gauge, Scrollbar, ScrollbarOrientation, ScrollbarState, block::Title,
    },
};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeName {
    #[default]
    Dark,
    Light,
    HighContrast,
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub focused_border:       Style,
    pub gauge:                Style,
    pub list_cursor:          Style,
    pub tree_cursor:          Style,
    pub wrong_answer:         Style,
    pub unsolved:             Style,
    pub solved:               Style,
    pub not_started:          Style,
    pub default:              Style,
    pub highlighted:          Style,
    pub error:                Style,
    pub default_tree_item:    Style,
    pub unproven_tree_item:   Style,
    pub proven_requirement:   Style,
    pub unproven_requirement: Style,
    pub skipped_requirement:  Style,
    pub solution_answer:      Style,
    pub solution_term:        Style,
    pub solution_goal:        Style,
    pub popup_border:         Style,
}

impl Theme {
    pub fn from_name(name: ThemeName) -> Self {
        match name {
            ThemeName::Dark => Self::dark(),
            ThemeName::Light => Self::light(),
            ThemeName::HighContrast => Self::high_contrast(),
        }
    }

    pub fn dark() -> Self {
        Self {
            focused_border:       Style::new().fg(Color::Cyan),
            gauge:                Style::new().fg(Color::Blue).bg(Color::DarkGray),
            list_cursor:          Style::new().underlined(),
            tree_cursor:          Style::new()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
            wrong_answer:         Style::new().fg(Color::Red),
            unsolved:             Style::new().fg(Color::Yellow),
            solved:               Style::new().fg(Color::Green),
            not_started:          Style::new(),
            default:              Style::new(),
            highlighted:          Style::new().fg(Color::LightBlue).bold(),
            error:                Style::new().fg(Color::Red).bold(),
            default_tree_item:    Style::new(),
            unproven_tree_item:   Style::new().crossed_out().dim(),
            proven_requirement:   Style::new().fg(Color::Green).bold(),
            unproven_requirement: Style::new().fg(Color::Red).bold(),
            skipped_requirement:  Style::new().fg(Color::Gray).bold(),
            solution_answer:      Style::new().fg(Color::Green).italic(),
            solution_term:        Style::new().fg(Color::Yellow),
            solution_goal:        Style::new().fg(Color::Cyan).bold(),
            popup_border:         Style::new().fg(Color::Red).bold(),
        }
    }

    pub fn light() -> Self {
        Self {
            focused_border:       Style::new().fg(Color::Blue),
            gauge:                Style::new().fg(Color::Blue).bg(Color::Gray),
            list_cursor:          Style::new().underlined(),
            tree_cursor:          Style::new()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            wrong_answer:         Style::new().fg(Color::Red),
            unsolved:             Style::new().fg(Color::Rgb(0xb5, 0x7a, 0x00)),
            solved:               Style::new().fg(Color::Rgb(0x00, 0x80, 0x00)),
            not_started:          Style::new().fg(Color::Black),
            default:              Style::new().fg(Color::Black),
            highlighted:          Style::new().fg(Color::Blue).bold(),
            error:                Style::new().fg(Color::Red).bold(),
            default_tree_item:    Style::new().fg(Color::Black),
            unproven_tree_item:   Style::new().crossed_out().dim(),
            proven_requirement:   Style::new().fg(Color::Rgb(0x00, 0x80, 0x00)).bold(),
            unproven_requirement: Style::new().fg(Color::Red).bold(),
            skipped_requirement:  Style::new().fg(Color::DarkGray).bold(),
            solution_answer:      Style::new().fg(Color::Rgb(0x00, 0x80, 0x00)).italic(),
            solution_term:        Style::new().fg(Color::Rgb(0xb5, 0x7a, 0x00)),
            solution_goal:        Style::new().fg(Color::Blue).bold(),
            popup_border:         Style::new().fg(Color::Red).bold(),
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            focused_border:       Style::new().fg(Color::White).bold(),
            gauge:                Style::new().fg(Color::White).bg(Color::Black),
            list_cursor:          Style::new().underlined().bold(),
            tree_cursor:          Style::new()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
            wrong_answer:         Style::new().fg(Color::Red).bold(),
            unsolved:             Style::new().fg(Color::Yellow).bold(),
            solved:               Style::new().fg(Color::Green).bold(),
            not_started:          Style::new().fg(Color::White),
            default:              Style::new().fg(Color::White),
            highlighted:          Style::new().fg(Color::White).bold().underlined(),
            error:                Style::new().fg(Color::Red).bold(),
            default_tree_item:    Style::new().fg(Color::White),
            unproven_tree_item:   Style::new().crossed_out().dim(),
            proven_requirement:   Style::new().fg(Color::Green).bold(),
            unproven_requirement: Style::new().fg(Color::Red).bold(),
            skipped_requirement:  Style::new().fg(Color::Gray).bold(),
            solution_answer:      Style::new().fg(Color::Green).bold().italic(),
            solution_term:        Style::new().fg(Color::Yellow).bold(),
            solution_goal:        Style::new().fg(Color::Cyan).bold(),
            popup_border:         Style::new().fg(Color::Red).bold(),
        }
    }

    pub fn block<'a>(&self, focused: bool, title: impl Into<Title<'a>>) -> Block<'a> {
        let style = if focused {
            self.focused_border
        } else {
            Style::new()
        };
        Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(title)
    }

    pub fn gauge<'a>(&self, title: impl Into<Title<'a>>) -> Gauge<'a> {
        let title = self.block(false, title);
        Gauge::default().block(title).gauge_style(self.gauge)
    }

    pub fn scrollbar(&self) -> Scrollbar<'static> {
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
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

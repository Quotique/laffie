use ratatui::{
    prelude::*,
    widgets::{Paragraph, Widget},
};

use solver::rule::SharedRule;

use crate::theme::Theme;

pub struct RuleWindow {
    pub rule:       SharedRule,
    pub is_focused: bool,
}

impl Widget for RuleWindow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        <Paragraph as Widget>::render(
            Paragraph::new(self.rule.to_string())
                .block(self.theme().block(self.is_focused, "Detailed")),
            area,
            buf,
        );
    }
}

impl RuleWindow {
    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

use ratatui::{
    prelude::*,
    widgets::{Paragraph, Widget},
};

use solver::rule::SharedRule;

use crate::theme::Theme;

pub struct RuleWindow {
    pub rule: SharedRule,
}

impl Widget for RuleWindow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        <Paragraph as Widget>::render(Paragraph::new(self.rule.to_string()), area, buf);
    }
}

impl RuleWindow {
    fn _theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

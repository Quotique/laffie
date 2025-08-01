use std::fmt::Display;

use ratatui::{
    prelude::*,
    widgets::{Paragraph, Widget},
};

use solver::rule::{RuleAttr, SharedRule};

use crate::theme::Theme;

pub struct RuleWindow {
    pub rule: SharedRule,
}

impl Widget for RuleWindow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let default = self.theme().default();
        let mut lines = vec![
            self.pair_line("", &self.rule.term, self.theme().highlighted()),
            Line::default(),
        ];
        lines.push(self.pair_line("Requirements", "", default));
        for r in &self.rule.requirements {
            lines.push(Line::from(format!(" {r}")));
        }
        lines.extend([
            Line::default(),
            self.pair_line(
                "Purpose: ",
                self.rule
                    .attribute(&RuleAttr::Purpose)
                    .next()
                    .map(|x| format!("{x:?}"))
                    .unwrap_or_default(),
                default,
            ),
            self.pair_line("Level: ", self.rule.level, default),
            self.pair_line("Trigger: ", &self.rule.symbol, default),
            Line::default(),
            self.pair_line("Attributes:", "", default),
        ]);
        for (k, v) in { self.rule.attrs.iter() }
            .filter(|(k, _)| ![RuleAttr::Level, RuleAttr::Purpose].contains(k))
        {
            lines.push(Line::from(format!(" {k:?}: {v:?}")));
        }

        <Paragraph as Widget>::render(Paragraph::new(lines), area, buf);
    }
}

impl RuleWindow {
    fn pair_line<'b>(&self, k: &'b str, v: impl Display, v_style: Style) -> Line<'b> {
        let highlighted = self.theme().highlighted();
        Line::from(vec![
            Span::styled(k, highlighted),
            Span::styled(v.to_string(), v_style),
        ])
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

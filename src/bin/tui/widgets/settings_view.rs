use ratatui::{
    prelude::*,
    widgets::{List, ListState, StatefulWidget},
};

use crate::{settings::Settings, theme::Theme};

pub struct SettingsView<'a> {
    pub settings: &'a Settings,
    pub theme:    &'a Theme,
}

impl<'a> StatefulWidget for SettingsView<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let lines: Vec<Line<'static>> = vec![
            self.kv("symbols", self.settings.symbols.display()),
            self.kv("tasks", self.settings.tasks.display()),
            self.kv("exec_deadline", self.settings.exec_deadline),
            self.kv("solve_parallelism", self.settings.solve_parallelism),
            self.kv("theme", format!("{:?}", self.settings.theme).to_lowercase()),
        ];
        let list = List::new(lines).highlight_style(self.theme.list_cursor);
        StatefulWidget::render(list, area, buf, state);
    }
}

impl<'a> SettingsView<'a> {
    fn kv(&self, key: &'static str, value: impl ToString) -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {key:20}"), self.theme.highlighted),
            Span::raw(value.to_string()),
        ])
    }
}

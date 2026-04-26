use ratatui::{
    prelude::*,
    widgets::{List, ListState, StatefulWidget},
};

use crate::{settings::Settings, theme::Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingField {
    Symbols,
    Tasks,
    ExecDeadline,
    SolveParallelism,
    Theme,
}

pub const FIELDS: &[SettingField] = &[
    SettingField::Symbols,
    SettingField::Tasks,
    SettingField::ExecDeadline,
    SettingField::SolveParallelism,
    SettingField::Theme,
];

pub struct SettingsView<'a> {
    pub settings: &'a Settings,
    pub theme:    &'a Theme,
}

impl<'a> StatefulWidget for SettingsView<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let lines: Vec<Line<'static>> = FIELDS.iter().map(|f| self.field_line(*f)).collect();
        let list = List::new(lines).highlight_style(self.theme.list_cursor);
        StatefulWidget::render(list, area, buf, state);
    }
}

impl<'a> SettingsView<'a> {
    fn field_line(&self, field: SettingField) -> Line<'static> {
        let (key, value) = match field {
            SettingField::Symbols => ("symbols", self.settings.symbols.display().to_string()),
            SettingField::Tasks => ("tasks", self.settings.tasks.display().to_string()),
            SettingField::ExecDeadline => {
                ("exec_deadline", self.settings.exec_deadline.to_string())
            }
            SettingField::SolveParallelism => (
                "solve_parallelism",
                self.settings.solve_parallelism.to_string(),
            ),
            SettingField::Theme => ("theme", format!("{:?}", self.settings.theme).to_lowercase()),
        };
        Line::from(vec![
            Span::styled(format!("  {key:20}"), self.theme.highlighted),
            Span::raw(value),
        ])
    }
}

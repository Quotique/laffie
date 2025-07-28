use std::{
    io,
    iter::once,
    path::{Path, PathBuf},
    sync::Arc,
};

use derive_more::Display;
use ratatui::{
    prelude::*,
    text::Line,
    widgets::{ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use parser::DirectoryParser;

use super::{popup::Popup, rules::Rules, tasks::Tasks};

pub struct State {
    pub current_tab: Tab,

    rules_path: PathBuf,
    tasks_path: PathBuf,

    rules: Rules,
    tasks: Tasks,

    popup: Option<Popup<'static>>,
}

#[derive(Debug, Clone, Copy, Display, Eq, PartialEq)]
pub enum Tab {
    Rules,
    Tasks,
    Tracing,
    // Setting,
}

impl State {
    pub fn try_new(
        exec_deadline: usize,
        symbols_dir: impl AsRef<Path>,
        tasks_dir: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let parser = DirectoryParser::new(symbols_dir.as_ref(), tasks_dir.as_ref());

        let rules = parser.load_rules().map(Arc::new)?;
        let tasks = parser.load_tasks()?;

        Ok(State {
            current_tab: Tab::Rules,

            rules_path: symbols_dir.as_ref().into(),
            tasks_path: tasks_dir.as_ref().into(),

            rules: Rules::new(rules.clone()),
            tasks: Tasks::new(exec_deadline, rules, tasks),

            popup: None,
        })
    }

    pub fn reload(&mut self) {
        if self.popup.is_some() {
            return;
        }

        let parser = DirectoryParser::new(&self.rules_path, &self.tasks_path);

        match parser.load_rules().map(Arc::new) {
            Ok(rules) => {
                self.rules = Rules::new(rules.clone());
                self.tasks.replace_rules(rules);
            }
            Err(e) => {
                self.popup = Some(Popup::new(
                    Line::from(Span::from("Error".to_owned())),
                    Paragraph::new(
                        once(Line::from(Span::from("Error or rules update!".to_owned())))
                            .chain(
                                e.to_string()
                                    .lines()
                                    .map(|x| Line::from(Span::from(format!("|{x}")))),
                            )
                            .chain(once(Line::from(Span::from(
                                "Rules not updated!".to_owned(),
                            ))))
                            .collect::<Vec<_>>(),
                    ),
                ))
            }
        };
    }

    #[inline]
    pub fn solve_all(&mut self) {
        if self.popup.is_some() {
            return;
        }

        if self.current_tab == Tab::Tasks {
            self.tasks.solve_all();
        }
    }

    #[inline]
    pub fn solve(&mut self) {
        if self.popup.is_some() {
            return;
        }

        if self.current_tab == Tab::Tasks {
            self.tasks.solve();
        }
    }

    pub fn next(&mut self) {
        if self.popup.is_some() {
            return;
        }

        match self.current_tab {
            Tab::Rules => self.rules.select_next(),
            Tab::Tasks => self.tasks.select_next(),
            Tab::Tracing => {
                let _ = self.tasks.tracing().map(|x| x.select_next());
            }
        }
    }

    pub fn previous(&mut self) {
        if self.popup.is_some() {
            return;
        }

        match self.current_tab {
            Tab::Rules => self.rules.select_previous(),
            Tab::Tasks => self.tasks.select_previous(),
            Tab::Tracing => {
                let _ = self.tasks.tracing().map(|x| x.select_previous());
            }
        }
    }

    pub fn left(&mut self) {
        if self.popup.is_some() {
            return;
        }

        match self.current_tab {
            Tab::Rules => self.rules.left(),
            Tab::Tasks => self.tasks.left(),
            Tab::Tracing => {
                let _ = self.tasks.tracing().map(|x| x.left());
            }
        }
    }

    pub fn right(&mut self) {
        if self.popup.is_some() {
            return;
        }

        match self.current_tab {
            Tab::Rules => self.rules.right(),
            Tab::Tasks => self.tasks.right(),
            Tab::Tracing => {
                let _ = self.tasks.tracing().map(|x| x.right());
            }
        }
    }

    pub fn toggle(&mut self) {
        if self.popup.is_some() {
            let _ = self.popup.take();
        }

        match self.current_tab {
            Tab::Tracing => {
                let _ = self.tasks.tracing().map(|x| x.toggle());
            }
            Tab::Tasks => self.tasks.toggle(),
            _ => {}
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        match self.current_tab {
            Tab::Rules => self.rules.draw(frame, area),
            Tab::Tasks => self.tasks.draw(frame, area),
            Tab::Tracing => {
                let _ = self.tasks.tracing().map(|x| x.draw(frame, area));
            }
        }

        if let Some(popup) = self.popup.as_mut() {
            popup.draw(frame, area);
        }
    }
}

impl Tab {
    pub const MAX: usize = 2;
}

impl From<Tab> for usize {
    fn from(val: Tab) -> Self {
        match val {
            Tab::Rules => 0,
            Tab::Tasks => 1,
            Tab::Tracing => 2,
            // Tab::Setting => 3,
        }
    }
}

impl From<usize> for Tab {
    fn from(value: usize) -> Self {
        match value {
            0 => Tab::Rules,
            1 => Tab::Tasks,
            2 => Tab::Tracing,
            _ => unimplemented!(),
            // _ => Tab::Setting,
        }
    }
}

pub fn border_focus() -> Style {
    Style::new().fg(Color::Cyan)
}

pub fn border_unfocus() -> Style {
    Style::new()
}

pub fn draw_scrollbar(frame: &mut Frame, area: Rect, len: usize, pos: usize) {
    let mut scrollbar_state = ScrollbarState::new(len).position(pos);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        area.inner(Margin {
            vertical:   1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
}

pub fn default_state() -> ListState {
    let mut state = ListState::default();
    state.select(Some(0));
    state
}

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
    widgets::{ListState, Paragraph},
};

use parser::DirectoryParser;

use super::{popup::Popup, rules::Rules, tasks::Tasks};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    SwitchTab(usize),
    None,
    Solve,
    SolveAll,
    Down,
    Up,
    Left,
    Right,
    Toggle,
    Reload,
}

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

    pub fn process(&mut self, command: Command) {
        if self.popup.is_some() {
            return;
        }

        match self.current_tab {
            Tab::Rules => self.rules.process(command),
            Tab::Tasks => self.tasks.process(command),
            Tab::Tracing => {
                if let Some(tracing) = self.tasks.tracing() {
                    tracing.process(command);
                }
            }
        }

        match command {
            Command::Reload => self.reload(),
            Command::SwitchTab(num) => {
                self.current_tab = Tab::from(num);
            }
            _ => {}
        }
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

pub fn default_state() -> ListState {
    let mut state = ListState::default();
    state.select(Some(0));
    state
}

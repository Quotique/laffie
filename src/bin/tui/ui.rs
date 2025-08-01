use std::{collections::HashMap, io, iter::once};

use derive_more::Display;
use ratatui::{
    prelude::*,
    widgets::{ListState, Paragraph},
};

use super::{pane::Pane, popup::Popup, settings::Settings, state::State};
use crate::pane::WidgetType;

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

pub struct Ui {
    panes: HashMap<Tab, Pane>,
    popup: Option<Popup<'static>>,

    pub current_tab: Tab,

    state: State,
}

#[derive(Debug, Clone, Copy, Display, Hash, Eq, PartialEq)]
pub enum Tab {
    Rules,
    Tasks,
    Tracing,
    // Setting,
}

impl Ui {
    pub fn try_new(settings: Settings) -> io::Result<Self> {
        let state = State::try_new(settings)?;

        let panes = HashMap::from_iter([
            (
                Tab::Rules,
                Pane::from_iter([
                    (WidgetType::RulesList, Constraint::Percentage(40)),
                    (WidgetType::RuleWindow, Constraint::Percentage(60)),
                ]),
            ),
            (
                Tab::Tasks,
                Pane::from_iter([
                    (WidgetType::TasksList, Constraint::Percentage(40)),
                    (WidgetType::Solution, Constraint::Percentage(60)),
                ]),
            ),
            (
                Tab::Tracing,
                Pane::from_iter([
                    (WidgetType::TracingTree, Constraint::Percentage(50)),
                    (WidgetType::TracingWindow, Constraint::Percentage(50)),
                ]),
            ),
        ]);

        Ok(Ui {
            current_tab: Tab::Rules,
            panes,
            state,
            popup: None,
        })
    }

    pub fn process(&mut self, command: Command) {
        if self.popup.is_some() {
            return;
        }

        if let Some(pane) = self.panes.get_mut(&self.current_tab) {
            pane.process(&mut self.state, command)
        }

        match command {
            Command::Reload => {
                if let Err(e) = self.state.reload() {
                    let err_text = Paragraph::new(
                        once(Line::from("Error or rules update!"))
                            .chain(e.to_string().lines().map(|x| Line::from(format!("|{x}"))))
                            .chain(once(Line::from("Rules not updated!")))
                            .collect::<Vec<_>>(),
                    );
                    self.popup = Some(Popup::new(Line::from("Error"), err_text))
                }
            }
            Command::SwitchTab(num) => {
                self.current_tab = Tab::from(num);
            }
            _ => {}
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(pane) = self.panes.get(&self.current_tab) {
            frame.render_stateful_widget(pane.clone(), area, &mut self.state)
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

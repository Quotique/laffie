use std::sync::Arc;

use derive_more::Display;
use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{
        Block, Borders, List, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use mcore::{
    rule::RulesEngine,
    task::{DumperConfig, Solution, EXECUTION_DEADLINE_DEFAULT},
};
use parser::DirectoryParser;
use utils::VecDisplay;
use view::View;

pub struct Status {
    pub current_tab: Tab,

    pub rules:          Arc<RulesEngine>,
    rules_state:        ListState,
    rules_focused_pane: usize,

    pub tasks:          Vec<TaskStatus>,
    tasks_state:        ListState,
    tasks_focused_pane: usize,
}

pub struct TaskStatus {
    solution: Solution,
    solved:   bool,
    scroll:   ListState,
}

#[derive(Debug, Clone, Copy, Display, Eq, PartialEq)]
pub enum Tab {
    Rules,
    Tasks,
    Tracing,
    Setting,
}

impl Status {
    pub fn new() -> Self {
        let parser = DirectoryParser::new("symbols", "tasks");

        let rules = Arc::new(parser.load_rules().unwrap());
        let tasks = parser.load_tasks().unwrap();

        Status {
            current_tab: Tab::Rules,
            tasks: tasks
                .into_iter()
                .map(|x| TaskStatus {
                    solution: Solution::new(
                        x,
                        rules.clone(),
                        DumperConfig {
                            sink:     "none".into(),
                            filename: "/dev/null".into(),
                        }
                        .build(),
                        EXECUTION_DEADLINE_DEFAULT,
                        Default::default(),
                    ),
                    solved:   false,
                    scroll:   Self::default_state(),
                })
                .collect(),
            rules,
            rules_state: Self::default_state(),
            rules_focused_pane: 0,
            tasks_state: Self::default_state(),
            tasks_focused_pane: 0,
        }
    }

    fn default_state() -> ListState {
        let mut state = ListState::default();
        state.select(Some(0));
        state
    }

    pub fn solve(&mut self) {
        if self.current_tab == Tab::Tasks {
            let task = self
                .tasks
                .get_mut(self.tasks_state.selected().unwrap())
                .unwrap();
            let _ = task.solution.solve();
            task.solved = true;
        }
    }

    pub fn next(&mut self) {
        match self.current_tab {
            Tab::Rules => self.rules_state.select_next(),
            Tab::Tasks => {
                if self.tasks_focused_pane == 0 {
                    self.tasks_state.select_next()
                } else {
                    self.tasks
                        .get_mut(self.tasks_state.selected().unwrap())
                        .unwrap()
                        .scroll
                        .select_next()
                }
            }
            _ => (),
        }
    }

    pub fn previous(&mut self) {
        match self.current_tab {
            Tab::Rules => self.rules_state.select_previous(),
            Tab::Tasks => {
                if self.tasks_focused_pane == 0 {
                    self.tasks_state.select_previous()
                } else {
                    self.tasks
                        .get_mut(self.tasks_state.selected().unwrap())
                        .unwrap()
                        .scroll
                        .select_previous()
                }
            }
            _ => (),
        }
    }

    pub fn left(&mut self) {
        match self.current_tab {
            Tab::Rules => self.rules_focused_pane = 0,
            Tab::Tasks => self.tasks_focused_pane = 0,
            _ => (),
        }
    }

    pub fn right(&mut self) {
        match self.current_tab {
            Tab::Rules => self.rules_focused_pane = 1,
            Tab::Tasks => self.tasks_focused_pane = 1,
            _ => (),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        match self.current_tab {
            Tab::Rules => self.draw_rules(frame, area),
            Tab::Tasks => self.draw_tasks(frame, area),
            Tab::Tracing => self.draw_tracing(frame, area),
            _ => {
                let horizontal_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
                    .split(area);

                let items = ["Item 1", "Item 2", "Item 3"];
                let list = List::new(items).highlight_symbol(">>").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Here is a list of items"),
                );

                frame.render_stateful_widget(list, horizontal_layout[0], &mut self.rules_state);

                let right_block = Block::default()
                    .borders(Borders::ALL)
                    .title("Here is a text");
                frame.render_widget(
                    Paragraph::new("Hello Ratatui! (press 'q' to quit)")
                        .yellow()
                        .on_blue()
                        .block(right_block),
                    horizontal_layout[1],
                );
            }
        }
    }

    fn draw_tracing(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
    }

    fn draw_tasks(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let list = List::new(self.tasks.iter().map(|x| {
            format!(
                "{} {}",
                x.solution.task.purpose,
                VecDisplay(&x.solution.task.conditions)
            )
        }))
        .highlight_symbol("> ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.pane_style(Tab::Tasks, 0))
                .title("Tasks"),
        );

        frame.render_stateful_widget(list, layout[0], &mut self.tasks_state);
        Self::draw_scrollbar(
            frame,
            layout[0],
            self.tasks.len(),
            self.tasks_state.selected().unwrap(),
        );

        let pane_style = self.pane_style(Tab::Tasks, 1);
        let task = self
            .tasks
            .get_mut(self.tasks_state.selected().unwrap())
            .unwrap();
        let solution = if task.solved {
            format!("Solution\n{}", View::try_from(&task.solution).unwrap())
        } else {
            "Press s to solve".to_owned()
        };

        let solution_lines: Vec<String> =
            format!("Conditions:\n{}\n\n{}", task.solution.task, solution)
                .split('\n')
                .map(|x| x.to_owned())
                .collect();
        frame.render_stateful_widget(
            List::new(solution_lines.iter().cloned())
                .highlight_style(Style::new().underlined())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(pane_style)
                        .title("Detailed"),
                ),
            layout[1],
            &mut task.scroll,
        );
        Self::draw_scrollbar(
            frame,
            layout[1],
            solution_lines.len(),
            task.scroll.selected().unwrap(),
        );
    }

    fn draw_rules(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let items: Vec<_> = self.rules.iter().collect();
        let list = List::new(items.iter().map(|x| x.term.to_string()))
            .highlight_symbol("> ")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.pane_style(Tab::Rules, 0))
                    .title("Rules"),
            );

        frame.render_stateful_widget(list, layout[0], &mut self.rules_state);
        Self::draw_scrollbar(
            frame,
            layout[0],
            items.len(),
            self.rules_state.selected().unwrap(),
        );

        frame.render_widget(
            Paragraph::new(
                items
                    .get(self.rules_state.selected().unwrap())
                    .unwrap()
                    .to_string(),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.pane_style(Tab::Rules, 1))
                    .title("Detailed"),
            ),
            layout[1],
        );
    }

    fn pane_style(&self, tab: Tab, pane: usize) -> Style {
        match tab {
            Tab::Rules => {
                if self.rules_focused_pane == pane {
                    border_focus()
                } else {
                    border_unfocus()
                }
            }
            Tab::Tasks => {
                if self.tasks_focused_pane == pane {
                    border_focus()
                } else {
                    border_unfocus()
                }
            }
            _ => Style::new(),
        }
    }

    fn draw_scrollbar(frame: &mut Frame, area: Rect, len: usize, pos: usize) {
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
}

impl Tab {
    pub const MAX: usize = 3;
}

impl From<Tab> for usize {
    fn from(val: Tab) -> Self {
        match val {
            Tab::Rules => 0,
            Tab::Tasks => 1,
            Tab::Tracing => 2,
            Tab::Setting => 3,
        }
    }
}

impl From<usize> for Tab {
    fn from(value: usize) -> Self {
        match value {
            0 => Tab::Rules,
            1 => Tab::Tasks,
            2 => Tab::Tracing,
            _ => Tab::Setting,
        }
    }
}

fn border_focus() -> Style {
    Style::new().fg(Color::Cyan)
}

fn border_unfocus() -> Style {
    Style::new()
}

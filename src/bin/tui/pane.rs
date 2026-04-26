use ratatui::{prelude::*, widgets::StatefulWidget};

use utils::IndexedTree;

use crate::{
    state::State,
    strings,
    theme::{Theme, ThemeName},
    widgets::{
        rule_window::RuleWindow, rules_list::RulesList, settings_view::SettingsView,
        solution_window::SolutionWindow, tasks_list::TasksList,
        tracing_navigation::TracingNavigation, tracing_window::TracingWindow,
    },
};

use super::ui::Command;

const PAGE_STEP: usize = 10;

#[derive(Debug, Clone, Copy)]
pub struct KeyHint {
    pub key:   &'static str,
    pub label: &'static str,
}

pub trait WidgetCommands {
    fn keys(&self) -> &'static [KeyHint];
    fn handle(&self, state: &mut State, command: Command) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetType {
    RulesList,
    RuleWindow,
    TasksList,
    Solution,
    TracingNavigation,
    TracingWindow,
    SettingsView,
}

#[derive(Debug, Clone)]
pub struct Pane {
    pub layout:       Vec<(WidgetType, Constraint)>,
    pub focused_pane: usize,
    pub theme:        Theme,
}

impl Pane {
    pub fn new(layout: Vec<(WidgetType, Constraint)>, theme: Theme) -> Self {
        Pane {
            layout,
            focused_pane: 0,
            theme,
        }
    }
}

impl StatefulWidget for Pane {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(self.layout.iter().map(|x| x.1))
            .split(area);

        for (num, (widget, area)) in { self.layout.iter().map(|x| x.0) }
            .zip(areas.iter())
            .enumerate()
        {
            let is_focused = self.focused_pane == num;
            match widget {
                WidgetType::RulesList => {
                    let items = state.filtered_rules();
                    let title = if state.rules_filter.is_empty() {
                        strings::pane_title::RULES.to_string()
                    } else {
                        format!("{} /{}", strings::pane_title::RULES, state.rules_filter)
                    };
                    let rules_list = RulesList { items };
                    let block = self.theme.block(is_focused, title.as_str());
                    let inner = block.inner(*area);
                    block.render(*area, buf);
                    rules_list.render(inner, buf, &mut state.rules_pos);
                }
                WidgetType::RuleWindow => {
                    let block = self.theme.block(is_focused, strings::pane_title::DETAILED);
                    let inner = block.inner(*area);
                    block.render(*area, buf);
                    if let Some(idx) = state.rules_pos.selected() &&
                        let Some(rule) = state.filtered_rules().into_iter().nth(idx)
                    {
                        RuleWindow {
                            rule,
                            theme: &self.theme,
                        }
                        .render(inner, buf);
                    }
                }
                WidgetType::TasksList => {
                    let task_list = TasksList {
                        tasks_index: &state.tasks,
                        theme:       &self.theme,
                    };
                    let block = self.theme.block(is_focused, strings::pane_title::TASKS);
                    let inner = block.inner(*area);
                    block.render(*area, buf);
                    task_list.render(inner, buf, &mut state.tasks_pos);
                }
                WidgetType::Solution => {
                    let block = self.theme.block(is_focused, strings::pane_title::DETAILED);
                    let inner = block.inner(*area);
                    block.render(*area, buf);
                    let selected = state.tasks_pos.selected().last().cloned();
                    let solution = SolutionWindow {
                        tasks_index: &mut state.tasks,
                        dir_scroll: &mut state.dir_solution_pos,
                        theme: &self.theme,
                        selected,
                    };
                    solution.render(inner, buf, &mut ());
                }
                WidgetType::TracingNavigation => {
                    if let Some(task_state) = state.selected_task() {
                        let block = self.theme.block(is_focused, strings::pane_title::TRACING);
                        let inner = block.inner(*area);
                        block.render(*area, buf);
                        let nav = TracingNavigation { theme: &self.theme };
                        (&nav).render(inner, buf, task_state);
                    }
                }
                WidgetType::TracingWindow => {
                    if let Some(task_state) = state.selected_task() {
                        let block = self.theme.block(is_focused, strings::pane_title::DETAILED);
                        let inner = block.inner(*area);
                        block.render(*area, buf);
                        TracingWindow {
                            selected: task_state
                                .tracing_state
                                .last()
                                .and_then(|x| x.1.selected().last())
                                .cloned(),
                            theme:    &self.theme,
                        }
                        .render(inner, buf);
                    }
                }
                WidgetType::SettingsView => {
                    let block = self.theme.block(is_focused, strings::pane_title::SETTINGS);
                    let inner = block.inner(*area);
                    block.render(*area, buf);
                    SettingsView {
                        settings: &state.settings,
                        theme:    &self.theme,
                    }
                    .render(inner, buf, &mut state.settings_pos);
                }
            }
        }
    }
}

impl WidgetCommands for WidgetType {
    fn keys(&self) -> &'static [KeyHint] {
        match self {
            WidgetType::RulesList => &[
                KeyHint {
                    key:   "↑↓",
                    label: "select rule",
                },
                KeyHint {
                    key:   "→",
                    label: "details",
                },
                KeyHint {
                    key:   "/",
                    label: "filter",
                },
                KeyHint {
                    key:   "e",
                    label: "edit .sym",
                },
            ],
            WidgetType::RuleWindow => &[
                KeyHint {
                    key:   "↑↓",
                    label: "select rule",
                },
                KeyHint {
                    key:   "←",
                    label: "back to list",
                },
                KeyHint {
                    key:   "/",
                    label: "filter",
                },
            ],
            WidgetType::TasksList => &[
                KeyHint {
                    key:   "↑↓",
                    label: "navigate",
                },
                KeyHint {
                    key:   "Space",
                    label: "toggle dir",
                },
                KeyHint {
                    key:   "→",
                    label: "solution",
                },
                KeyHint {
                    key:   "s",
                    label: "solve selected",
                },
                KeyHint {
                    key:   "a",
                    label: "solve all",
                },
                KeyHint {
                    key:   "e",
                    label: "edit .pbl",
                },
            ],
            WidgetType::Solution => &[
                KeyHint {
                    key:   "↑↓",
                    label: "scroll",
                },
                KeyHint {
                    key:   "←",
                    label: "back to list",
                },
                KeyHint {
                    key:   "s",
                    label: "solve selected",
                },
                KeyHint {
                    key:   "a",
                    label: "solve all",
                },
            ],
            WidgetType::TracingNavigation => &[
                KeyHint {
                    key:   "↑↓",
                    label: "navigate",
                },
                KeyHint {
                    key:   "Space",
                    label: "toggle node",
                },
                KeyHint {
                    key:   "→",
                    label: "into",
                },
                KeyHint {
                    key:   "←",
                    label: "back",
                },
            ],
            WidgetType::TracingWindow => &[],
            WidgetType::SettingsView => &[
                KeyHint {
                    key:   "↑↓",
                    label: "navigate",
                },
                KeyHint {
                    key:   "←→",
                    label: "adjust",
                },
                KeyHint {
                    key:   "Space",
                    label: "cycle",
                },
                KeyHint {
                    key:   "Ctrl+S",
                    label: "save",
                },
            ],
        }
    }

    fn handle(&self, state: &mut State, command: Command) -> bool {
        match self {
            WidgetType::RulesList | WidgetType::RuleWindow => match command {
                Command::Down => {
                    state.rules_pos.select_next();
                    true
                }
                Command::Up => {
                    state.rules_pos.select_previous();
                    true
                }
                Command::PageDown => {
                    state.rules_pos.scroll_down_by(PAGE_STEP as u16);
                    true
                }
                Command::PageUp => {
                    state.rules_pos.scroll_up_by(PAGE_STEP as u16);
                    true
                }
                Command::Top => {
                    state.rules_pos.select_first();
                    true
                }
                Command::Bottom => {
                    state.rules_pos.select_last();
                    true
                }
                _ => false,
            },
            WidgetType::TracingNavigation => {
                let Some(task_state) = state.selected_task() else {
                    return false;
                };
                if let Some(tracing) = task_state.tracing_state.last_mut() {
                    match command {
                        Command::Down => {
                            tracing.1.key_down();
                            return true;
                        }
                        Command::Up => {
                            tracing.1.key_up();
                            return true;
                        }
                        Command::PageDown => {
                            tracing.1.scroll_down(PAGE_STEP);
                            return true;
                        }
                        Command::PageUp => {
                            tracing.1.scroll_up(PAGE_STEP);
                            return true;
                        }
                        Command::Top => {
                            tracing.1.select_first();
                            return true;
                        }
                        Command::Bottom => {
                            tracing.1.select_last();
                            return true;
                        }
                        Command::Toggle => {
                            tracing.1.toggle_selected();
                            return true;
                        }
                        _ => {}
                    }
                }
                match command {
                    Command::Left => {
                        if task_state.tracing_state.len() > 1 {
                            task_state.tracing_state.pop();
                        }
                        true
                    }
                    Command::Right => {
                        if let Some(term) = task_state
                            .tracing_state
                            .last()
                            .and_then(|x| x.1.selected().last()) &&
                            term.idx == 0
                        {
                            task_state
                                .tracing_state
                                .push((term.solution.clone(), Default::default()));
                        }
                        true
                    }
                    _ => false,
                }
            }
            WidgetType::TasksList => match command {
                Command::SolveAll => {
                    state.mark_to_solve(state.tasks.root().id());
                    true
                }
                Command::Solve => {
                    if let Some(selected) = state.tasks_pos.selected().last().cloned() {
                        state.mark_to_solve(selected);
                    }
                    true
                }
                Command::Down => {
                    state.tasks_pos.key_down();
                    true
                }
                Command::Up => {
                    state.tasks_pos.key_up();
                    true
                }
                Command::PageDown => {
                    state.tasks_pos.scroll_down(PAGE_STEP);
                    true
                }
                Command::PageUp => {
                    state.tasks_pos.scroll_up(PAGE_STEP);
                    true
                }
                Command::Top => {
                    state.tasks_pos.select_first();
                    true
                }
                Command::Bottom => {
                    state.tasks_pos.select_last();
                    true
                }
                Command::Toggle => {
                    state.tasks_pos.toggle_selected();
                    true
                }
                _ => false,
            },
            WidgetType::Solution => match command {
                Command::SolveAll => {
                    state.mark_to_solve(state.tasks.root().id());
                    true
                }
                Command::Solve => {
                    if let Some(selected) = state.tasks_pos.selected().last().cloned() {
                        state.mark_to_solve(selected);
                    }
                    true
                }
                Command::Down => {
                    state.solution_scroll_mut().scroll_down();
                    true
                }
                Command::Up => {
                    state.solution_scroll_mut().scroll_up();
                    true
                }
                Command::PageDown => {
                    state.solution_scroll_mut().scroll_page_down();
                    true
                }
                Command::PageUp => {
                    state.solution_scroll_mut().scroll_page_up();
                    true
                }
                Command::Top => {
                    state.solution_scroll_mut().scroll_to_top();
                    true
                }
                Command::Bottom => {
                    state.solution_scroll_mut().scroll_to_bottom();
                    true
                }
                _ => false,
            },
            WidgetType::TracingWindow => false,
            WidgetType::SettingsView => match command {
                Command::Down => {
                    state.settings_pos.select_next();
                    true
                }
                Command::Up => {
                    state.settings_pos.select_previous();
                    true
                }
                Command::Top => {
                    state.settings_pos.select_first();
                    true
                }
                Command::Bottom => {
                    state.settings_pos.select_last();
                    true
                }
                Command::Left | Command::Right => {
                    let inc = matches!(command, Command::Right);
                    adjust_setting(state, inc);
                    true
                }
                Command::Toggle => {
                    cycle_theme(state);
                    true
                }
                _ => false,
            },
        }
    }
}

impl Pane {
    pub fn keys(&self) -> &'static [KeyHint] {
        self.layout[self.focused_pane].0.keys()
    }

    pub fn focused(&self) -> WidgetType {
        self.layout[self.focused_pane].0
    }

    pub fn click(&mut self, col: u16, row: u16, body: Rect) {
        let areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(self.layout.iter().map(|(_, c)| *c))
            .split(body);
        let pos = Position::new(col, row);
        for (idx, area) in areas.iter().enumerate() {
            if area.contains(pos) {
                self.focused_pane = idx;
                return;
            }
        }
    }

    pub fn process(&mut self, state: &mut State, command: Command) {
        let len = self.layout.len();
        match command {
            Command::NextPane => {
                if len > 0 {
                    self.focused_pane = (self.focused_pane + 1) % len;
                }
                return;
            }
            Command::PrevPane => {
                if len > 0 {
                    self.focused_pane = (self.focused_pane + len - 1) % len;
                }
                return;
            }
            _ => {}
        }
        let focused = self.layout[self.focused_pane].0;
        if focused.handle(state, command) {
            return;
        }
        match command {
            Command::Left => {
                self.focused_pane = self.focused_pane.saturating_sub(1);
            }
            Command::Right => {
                self.focused_pane = (self.focused_pane + 1).min(len.saturating_sub(1));
            }
            _ => {}
        }
    }
}

const EXEC_DEADLINE_STEP: usize = 10_000;

fn adjust_setting(state: &mut State, increase: bool) {
    let Some(idx) = state.settings_pos.selected() else {
        return;
    };
    match idx {
        2 => {
            state.settings.exec_deadline = if increase {
                state
                    .settings
                    .exec_deadline
                    .saturating_add(EXEC_DEADLINE_STEP)
            } else {
                state
                    .settings
                    .exec_deadline
                    .saturating_sub(EXEC_DEADLINE_STEP)
                    .max(EXEC_DEADLINE_STEP)
            };
        }
        3 => {
            state.settings.solve_parallelism = if increase {
                state.settings.solve_parallelism.saturating_add(1)
            } else {
                state.settings.solve_parallelism.saturating_sub(1).max(1)
            };
        }
        4 => cycle_theme(state),
        _ => {}
    }
}

fn cycle_theme(state: &mut State) {
    let next = match state.settings.theme {
        ThemeName::Dark => ThemeName::Light,
        ThemeName::Light => ThemeName::HighContrast,
        ThemeName::HighContrast => ThemeName::Dark,
    };
    state.settings.theme = next;
}

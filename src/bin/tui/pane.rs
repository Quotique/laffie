use ratatui::{prelude::*, widgets::StatefulWidget};

use utils::IndexedTree;

use crate::{
    state::State,
    theme::Theme,
    widgets::{
        rule_window::RuleWindow, rules_list::RulesList, solution_window::SolutionWindow,
        tasks_list::TasksList, tracing_navigation::TracingNavigation,
        tracing_window::TracingWindow,
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
}

#[derive(Debug, Clone)]
pub struct Pane {
    pub layout:       Vec<(WidgetType, Constraint)>,
    pub focused_pane: usize,
}

impl FromIterator<(WidgetType, Constraint)> for Pane {
    fn from_iter<T: IntoIterator<Item = (WidgetType, Constraint)>>(iter: T) -> Self {
        Pane {
            layout:       FromIterator::from_iter(iter),
            focused_pane: 0,
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
                    let rules_list = RulesList {
                        engine: state.rules_engine.clone(),
                    };
                    let block = self.theme().block(is_focused, "Rules");
                    let inner = block.inner(*area);
                    block.render(*area, buf);
                    rules_list.render(inner, buf, &mut state.rules_pos);
                }
                WidgetType::RuleWindow => {
                    let block = self.theme().block(is_focused, "Detailed");
                    let inner = block.inner(*area);
                    block.render(*area, buf);
                    if let Some(idx) = state.rules_pos.selected() &&
                        let Some(rule) = state.rules_engine.iter().nth(idx)
                    {
                        RuleWindow { rule }.render(inner, buf);
                    }
                }
                WidgetType::TasksList => {
                    let task_list = TasksList {
                        tasks_index: &state.tasks,
                    };
                    let block = self.theme().block(is_focused, "Tasks");
                    let inner = block.inner(*area);
                    block.render(*area, buf);
                    task_list.render(inner, buf, &mut state.tasks_pos);
                }
                WidgetType::Solution => {
                    let block = self.theme().block(is_focused, "Detailed");
                    let solution = SolutionWindow {
                        tasks_index: &mut state.tasks,
                        selected:    state.tasks_pos.selected().last().cloned(),
                    };
                    let inner = block.inner(*area);
                    block.render(*area, buf);
                    solution.render(inner, buf, &mut ());
                }
                WidgetType::TracingNavigation => {
                    if let Some(task_state) = state.selected_task() {
                        let block = self.theme().block(is_focused, "Tracing");
                        let inner = block.inner(*area);
                        block.render(*area, buf);
                        TracingNavigation::default().render(inner, buf, task_state);
                    }
                }
                WidgetType::TracingWindow => {
                    if let Some(task_state) = state.selected_task() {
                        let block = self.theme().block(is_focused, "Detailed");
                        let inner = block.inner(*area);
                        block.render(*area, buf);
                        TracingWindow {
                            selected: task_state
                                .tracing_state
                                .last()
                                .and_then(|x| x.1.selected().last())
                                .cloned(),
                        }
                        .render(inner, buf);
                    }
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
                    if let Some(task_state) = state.selected_task() {
                        task_state.solution_pos.scroll_down();
                    }
                    true
                }
                Command::Up => {
                    if let Some(task_state) = state.selected_task() {
                        task_state.solution_pos.scroll_up();
                    }
                    true
                }
                Command::PageDown => {
                    if let Some(task_state) = state.selected_task() {
                        task_state.solution_pos.scroll_page_down();
                    }
                    true
                }
                Command::PageUp => {
                    if let Some(task_state) = state.selected_task() {
                        task_state.solution_pos.scroll_page_up();
                    }
                    true
                }
                Command::Top => {
                    if let Some(task_state) = state.selected_task() {
                        task_state.solution_pos.scroll_to_top();
                    }
                    true
                }
                Command::Bottom => {
                    if let Some(task_state) = state.selected_task() {
                        task_state.solution_pos.scroll_to_bottom();
                    }
                    true
                }
                _ => false,
            },
            WidgetType::TracingWindow => false,
        }
    }
}

impl Pane {
    pub fn keys(&self) -> &'static [KeyHint] {
        self.layout[self.focused_pane].0.keys()
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

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

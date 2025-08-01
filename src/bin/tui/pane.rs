use ratatui::{prelude::*, widgets::StatefulWidget};

use utils::IndexedTree;

use crate::{
    state::State,
    theme::Theme,
    widgets::{
        rule_window::RuleWindow, rules_list::RulesList, solution_window::SolutionWindow,
        tasks_list::TasksList, tracing_tree::TracingTree, tracing_window::TracingWindow,
    },
};

use super::ui::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetType {
    RulesList,
    RuleWindow,
    TasksList,
    Solution,
    TracingTree,
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
                    let rule_window = RuleWindow {
                        rule: { state.rules_engine.iter() }
                            .nth(state.rules_pos.selected().expect("missing selected"))
                            .expect("rule not found"),
                    };
                    let block = self.theme().block(is_focused, "Detailed");
                    let inner = block.inner(*area);
                    block.render(*area, buf);
                    rule_window.render(inner, buf);
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
                WidgetType::TracingTree => {
                    if let Some(task_state) = state.selected_task() {
                        let block = self.theme().block(is_focused, "Tracing");
                        let inner = block.inner(*area);
                        block.render(*area, buf);
                        TracingTree {
                            solution: task_state.solution.clone(),
                        }
                        .render(inner, buf, &mut task_state.tracing_pos);
                    }
                }
                WidgetType::TracingWindow => {
                    if let Some(task_state) = state.selected_task() {
                        let block = self.theme().block(is_focused, "Detailed");
                        let inner = block.inner(*area);
                        block.render(*area, buf);
                        TracingWindow {
                            selected: task_state.tracing_pos.selected().last().cloned(),
                        }
                        .render(inner, buf);
                    }
                }
            }
        }
    }
}

impl Pane {
    pub fn process(&mut self, state: &mut State, command: Command) {
        let focused = self.layout[self.focused_pane].0;
        match focused {
            WidgetType::RulesList | WidgetType::RuleWindow => match command {
                Command::Down => state.rules_pos.select_next(),
                Command::Up => state.rules_pos.select_previous(),
                Command::Left => self.focused_pane = 0,
                Command::Right => self.focused_pane = 1,
                _ => {}
            },
            WidgetType::TracingTree => {
                if let Some(task_state) = state.selected_task() {
                    let _ = match command {
                        Command::Down => task_state.tracing_pos.key_down(),
                        Command::Up => task_state.tracing_pos.key_up(),
                        Command::Left => task_state.tracing_pos.key_left(),
                        Command::Right => task_state.tracing_pos.key_right(),
                        Command::Toggle => task_state.tracing_pos.toggle_selected(),
                        _ => false,
                    };
                }
            }
            WidgetType::TasksList => {
                match command {
                    Command::SolveAll => state.solve_node_id(&state.tasks.root().id()),
                    Command::Solve => {
                        if let Some(selected) = state.tasks_pos.selected().last().cloned() {
                            state.solve_node_id(&selected);
                        }
                    }
                    Command::Down => {
                        state.tasks_pos.key_down();
                    }
                    Command::Up => {
                        state.tasks_pos.key_up();
                    }
                    Command::Left => self.focused_pane = 0,
                    Command::Right => self.focused_pane = 1,
                    Command::Toggle => {
                        state.tasks_pos.toggle_selected();
                    }
                    _ => {}
                };
            }
            WidgetType::Solution => {
                match command {
                    Command::SolveAll => state.solve_node_id(&state.tasks.root().id()),
                    Command::Solve => {
                        if let Some(selected) = state.tasks_pos.selected().last().cloned() {
                            state.solve_node_id(&selected);
                        }
                    }
                    Command::Down => {
                        if let Some(task_state) = state.selected_task() {
                            task_state.solution_pos.select_next()
                        }
                    }
                    Command::Up => {
                        if let Some(task_state) = state.selected_task() {
                            task_state.solution_pos.select_previous()
                        }
                    }
                    Command::Left => self.focused_pane = 0,
                    Command::Right => self.focused_pane = 1,
                    _ => {}
                };
            }
            _ => {}
        }
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

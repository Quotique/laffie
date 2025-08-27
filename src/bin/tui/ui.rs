use std::{
    collections::HashMap,
    io,
    iter::once,
    sync::Arc,
    thread::{spawn, JoinHandle},
};

use derive_more::Display;
use parking_lot::Mutex;
use ratatui::{
    prelude::*,
    widgets::{ListState, Paragraph, Wrap},
};

use solver::task::{SharedSolution, Solution, Solver, TracerHub};
use utils::{IndexedTree, TreeIndex};

use super::{
    pane::Pane,
    settings::Settings,
    state::{State, TasksNode},
};
use crate::{
    pane::WidgetType,
    widgets::{
        popup::Popup,
        solver_progress::{ProgressReporter, SolverProgress},
    },
};

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
    Cancel,
}

pub struct Ui {
    panes: HashMap<Tab, Pane>,

    error:    String,
    worker:   Option<JoinHandle<Vec<(TreeIndex, SharedSolution)>>>,
    progress: Arc<Mutex<SolverProgress>>,

    pub current_tab: Tab,
    state:           State,
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
            error: Default::default(),
            worker: None,
            progress: Mutex::new(SolverProgress::new(state.settings.exec_deadline)).into(),
            state,
        })
    }

    pub fn process_queue(&mut self) {
        if self.state.solve_queue.is_empty() {
            return;
        }

        let queue = { self.state.solve_queue.split_off(0) }
            .into_iter()
            .filter_map(|idx| {
                let TasksNode::Task(task) = self.state.tasks.get_mut(&idx).unwrap().data() else {
                    return None;
                };
                Some((idx, Solution::new(task.solution.task.clone())))
            })
            .collect::<Vec<_>>();

        self.progress.lock().current_cycles = 0;
        self.progress.lock().finished_tasks_count = 0;
        self.progress.lock().total_tasks_count = queue.len();

        let reporter = ProgressReporter(self.progress.clone());
        let rules = self.state.rules_engine.clone();
        let exec_deadline = self.state.settings.exec_deadline;
        self.worker = Some(spawn(move || {
            queue
                .into_iter()
                .map(|(idx, task)| {
                    let mut hub = TracerHub::default();
                    hub.add_custom(reporter.clone());

                    reporter.0.lock().current_task = Some(task.task.clone());
                    let solution = Solver::new(rules.clone()).solve(task.task, hub, exec_deadline);
                    reporter.0.lock().finished_tasks_count += 1;
                    reporter.0.lock().current_cycles = 0;
                    (idx, solution)
                })
                .collect::<Vec<_>>()
        }));
    }

    pub fn process(&mut self, command: Command) {
        if !self.error.is_empty() {
            return;
        }

        if self.worker.is_some() {
            self.progress.lock().process(command);
            return;
        }

        if let Some(pane) = self.panes.get_mut(&self.current_tab) {
            pane.process(&mut self.state, command)
        }

        match command {
            Command::Reload => {
                if let Err(e) = self.state.reload() {
                    self.error = e.to_string();
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

        if !self.error.is_empty() {
            let mut popup = Popup::new(Line::from("Error"));
            let inner = popup.inner(area);
            popup.draw(frame, area);

            let err_lines = self.error.lines().map(|x| Line::from(format!("|{x}")));
            let err_text = Paragraph::new(
                once(Line::from("Error on rules update!"))
                    .chain(err_lines)
                    .chain(once(Line::from("Rules not updated!")))
                    .collect::<Vec<_>>(),
            );
            err_text
                .wrap(Wrap { trim: true })
                .style(Style::new().white())
                .render(inner, frame.buffer_mut());
        }

        if let Some(handler) = self.worker.as_mut() {
            if !handler.is_finished() {
                self.progress.lock().draw(frame, area);
            } else {
                for (idx, solution) in self.worker.take().unwrap().join().unwrap() {
                    self.state.update_task_solution(&idx, solution);
                }
            }
        } else {
            self.process_queue();
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

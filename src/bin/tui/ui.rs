use std::{
    collections::HashMap,
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{JoinHandle, spawn},
};

use derive_more::Display;
use ratatui::{
    prelude::*,
    widgets::{ListState, Paragraph, Wrap},
};

use solver::task::{SharedSolution, Solution, Solver, TIME_LIMIT_DEFAULT, TracerHub};
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
        solver_progress::{ProgressEvent, ProgressReporter, SolverProgress},
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

    error:  String,
    worker: Option<JoinHandle<Vec<(TreeIndex, SharedSolution)>>>,

    progress:    SolverProgress,
    progress_tx: Sender<ProgressEvent>,
    progress_rx: Receiver<ProgressEvent>,
    cancel:      Arc<AtomicBool>,
    cycles:      Arc<AtomicUsize>,

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
                    (WidgetType::TracingNavigation, Constraint::Percentage(60)),
                    (WidgetType::TracingWindow, Constraint::Percentage(40)),
                ]),
            ),
        ]);

        let (progress_tx, progress_rx) = mpsc::channel();
        Ok(Ui {
            current_tab: Tab::Rules,
            panes,
            error: Default::default(),
            worker: None,
            progress: SolverProgress::new(state.settings.exec_deadline),
            progress_tx,
            progress_rx,
            cancel: Arc::new(AtomicBool::new(false)),
            cycles: Arc::new(AtomicUsize::new(0)),
            state,
        })
    }

    pub fn has_active_worker(&self) -> bool {
        self.worker.is_some()
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

        self.progress.reset(queue.len());
        self.cycles.store(0, Ordering::Relaxed);
        self.cancel.store(false, Ordering::Relaxed);

        let reporter = ProgressReporter {
            cancel: self.cancel.clone(),
            cycles: self.cycles.clone(),
        };
        let tx = self.progress_tx.clone();
        let cycles = self.cycles.clone();
        let rules = self.state.rules_engine.clone();
        let exec_deadline = self.state.settings.exec_deadline;
        self.worker = Some(spawn(move || {
            queue
                .into_iter()
                .map(|(idx, task)| {
                    let mut hub = TracerHub::default();
                    hub.add_custom(reporter.clone());

                    let _ = tx.send(ProgressEvent::TaskStarted(Box::new(task.task.clone())));
                    let solution = Solver::new(rules.clone()).solve(
                        task.task,
                        hub,
                        exec_deadline,
                        TIME_LIMIT_DEFAULT,
                    );
                    let _ = tx.send(ProgressEvent::TaskFinished);
                    cycles.store(0, Ordering::Relaxed);
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
            if matches!(command, Command::Cancel) {
                self.cancel.store(true, Ordering::Relaxed);
            }
            return;
        }

        if let Some(pane) = self.panes.get_mut(&self.current_tab) {
            pane.process(&mut self.state, command)
        }

        match command {
            Command::Reload => {
                if let Err(e) = self.state.reload() {
                    self.error = format!("Reload failed: {e}");
                }
            }
            Command::SwitchTab(num) => {
                self.current_tab = Tab::from(num);
            }
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        while let Ok(event) = self.progress_rx.try_recv() {
            self.progress.handle(event);
        }

        match self.worker.take() {
            Some(handler) if !handler.is_finished() => {
                self.worker = Some(handler);
            }
            Some(handler) => match handler.join() {
                Ok(results) => {
                    for (idx, solution) in results {
                        self.state.update_task_solution(&idx, solution);
                    }
                }
                Err(payload) => {
                    let msg = panic_message(payload);
                    self.error = format!("Solver thread crashed: {msg}");
                }
            },
            None => self.process_queue(),
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

            let err_text = Paragraph::new(
                self.error
                    .lines()
                    .map(|x| Line::from(format!("| {x}")))
                    .collect::<Vec<_>>(),
            );
            err_text
                .wrap(Wrap { trim: true })
                .style(Style::new().white())
                .render(inner, frame.buffer_mut());
        }

        if self.has_active_worker() {
            let cycles = self.cycles.load(Ordering::Relaxed);
            self.progress.draw(frame, area, cycles);
        }
    }
}

impl Tab {
    pub const ALL: &'static [Tab] = &[Tab::Rules, Tab::Tasks, Tab::Tracing];
}

impl From<Tab> for usize {
    fn from(val: Tab) -> Self {
        Tab::ALL.iter().position(|t| *t == val).unwrap_or(0)
    }
}

impl From<usize> for Tab {
    fn from(value: usize) -> Self {
        Tab::ALL.get(value).copied().unwrap_or(Tab::Rules)
    }
}

pub fn default_state() -> ListState {
    let mut state = ListState::default();
    state.select(Some(0));
    state
}

fn panic_message(payload: Box<dyn std::any::Any + Send + 'static>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panicked".to_string()
    }
}

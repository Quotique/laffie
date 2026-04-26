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
    widgets::{Block, Borders, Clear, ListState, Paragraph, Wrap},
};

use solver::task::{SharedSolution, Solution, Solver, TIME_LIMIT_DEFAULT, TracerHub};
use utils::{IndexedTree, TreeIndex};

use super::{
    pane::Pane,
    settings::Settings,
    state::{State, TasksNode},
};
use crate::{
    pane::{KeyHint, WidgetType},
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
    PageDown,
    PageUp,
    Top,
    Bottom,
    NextPane,
    PrevPane,
    Toggle,
    Reload,
    Cancel,
    ShowHelp,
    Dismiss,
    FilterEnter,
    FilterChar(char),
    FilterBackspace,
    FilterFinish,
}

pub struct Ui {
    panes: HashMap<Tab, Pane>,

    error:       String,
    show_help:   bool,
    filter_mode: bool,
    worker:      Option<JoinHandle<Vec<(TreeIndex, SharedSolution)>>>,

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
            show_help: false,
            filter_mode: false,
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

    pub fn is_filter_mode(&self) -> bool {
        self.filter_mode
    }

    pub fn click_in_body(&mut self, col: u16, row: u16, body: Rect) {
        if let Some(pane) = self.panes.get_mut(&self.current_tab) {
            pane.click(col, row, body);
        }
    }

    pub fn key_hints(&self) -> Vec<KeyHint> {
        if self.show_help {
            return vec![KeyHint {
                key:   "Esc/?",
                label: "close help",
            }];
        }
        if self.filter_mode {
            return vec![
                KeyHint {
                    key:   "Esc",
                    label: "cancel filter",
                },
                KeyHint {
                    key:   "Enter",
                    label: "apply",
                },
            ];
        }
        if !self.error.is_empty() {
            return vec![
                KeyHint {
                    key:   "Esc",
                    label: "dismiss",
                },
                KeyHint {
                    key:   "q",
                    label: "quit",
                },
            ];
        }
        if self.has_active_worker() {
            return vec![
                KeyHint {
                    key:   "c",
                    label: "cancel",
                },
                KeyHint {
                    key:   "q",
                    label: "quit",
                },
            ];
        }
        let mut hints: Vec<KeyHint> = Vec::new();
        if let Some(pane) = self.panes.get(&self.current_tab) {
            hints.extend(pane.keys().iter().copied());
        }
        hints.push(KeyHint {
            key:   "r",
            label: "reload",
        });
        hints.push(KeyHint {
            key:   "?",
            label: "help",
        });
        hints.push(KeyHint {
            key:   "q",
            label: "quit",
        });
        hints
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
        let rules = self.state.rules_engine.clone();
        let exec_deadline = self.state.settings.exec_deadline;
        let parallelism = self.state.settings.solve_parallelism.max(1);
        self.worker = Some(spawn(move || {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(parallelism)
                .build()
                .expect("failed to build rayon pool");
            pool.install(|| {
                use rayon::iter::{IntoParallelIterator, ParallelIterator};
                queue
                    .into_par_iter()
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
                        (idx, solution)
                    })
                    .collect::<Vec<_>>()
            })
        }));
    }

    pub fn process(&mut self, command: Command) {
        if self.show_help {
            if matches!(command, Command::Dismiss | Command::ShowHelp) {
                self.show_help = false;
            }
            return;
        }
        if !self.error.is_empty() {
            if matches!(command, Command::Dismiss) {
                self.error.clear();
            }
            return;
        }
        if matches!(command, Command::ShowHelp) {
            self.show_help = true;
            return;
        }

        if self.filter_mode {
            match command {
                Command::Dismiss => {
                    self.filter_mode = false;
                    self.state.rules_filter.clear();
                    self.state.rules_pos.select(Some(0));
                }
                Command::FilterFinish => {
                    self.filter_mode = false;
                }
                Command::FilterChar(c) => {
                    self.state.rules_filter.push(c);
                    self.state.rules_pos.select(Some(0));
                }
                Command::FilterBackspace => {
                    self.state.rules_filter.pop();
                    self.state.rules_pos.select(Some(0));
                }
                Command::Up |
                Command::Down |
                Command::PageUp |
                Command::PageDown |
                Command::Top |
                Command::Bottom => {
                    if let Some(pane) = self.panes.get_mut(&self.current_tab) {
                        pane.process(&mut self.state, command);
                    }
                }
                _ => {}
            }
            return;
        }

        if matches!(command, Command::Cancel) {
            if self.worker.is_some() {
                self.cancel.store(true, Ordering::Relaxed);
            }
            return;
        }

        if matches!(command, Command::FilterEnter) {
            let supports_filter = self
                .panes
                .get(&self.current_tab)
                .map(|p| matches!(p.focused(), WidgetType::RulesList | WidgetType::RuleWindow))
                .unwrap_or(false);
            if supports_filter {
                self.filter_mode = true;
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
            let queued = self.state.solve_queue.len();
            self.progress.draw(frame, area, cycles, queued);
        }

        if self.show_help {
            draw_help(frame, area);
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

const HELP_ENTRIES: &[(&str, &str)] = &[
    ("F1 / F2 / F3", "switch tab"),
    ("Tab / Shift+Tab", "next / previous panel"),
    ("← ↑ → ↓", "navigation"),
    ("PgUp / PgDn", "page up / down"),
    ("Ctrl+u / Ctrl+d", "page up / down"),
    ("Home / End", "jump to top / bottom"),
    ("Space / Enter", "toggle (tree node, etc.)"),
    ("/", "filter (Rules)"),
    ("s", "solve selected task"),
    ("a", "solve all tasks"),
    ("c", "cancel running solver"),
    ("r", "reload rules"),
    ("?", "toggle help"),
    ("Esc", "dismiss popup / cancel filter"),
    ("q", "quit"),
];

fn draw_help(frame: &mut Frame, area: Rect) {
    let popup_area = Rect {
        x:      area.width / 8,
        y:      area.height / 6,
        width:  area.width * 3 / 4,
        height: (area.height * 2 / 3).max(8),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title("Help");
    let inner = block.inner(popup_area);
    Clear.render(popup_area, frame.buffer_mut());
    block.render(popup_area, frame.buffer_mut());

    let key_style = Style::default().fg(Color::Red);
    let lines: Vec<Line> = HELP_ENTRIES
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("  {k:18}"), key_style),
                Span::raw(*v),
            ])
        })
        .collect();

    Paragraph::new(lines).render(inner, frame.buffer_mut());
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

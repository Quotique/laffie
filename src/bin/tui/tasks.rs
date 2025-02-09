use std::sync::Arc;

use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{Block, Borders, List, ListState},
};

use solver::{
    rule::RulesEngine,
    task::{DumperConfig, Solver, Task, EXECUTION_DEADLINE_DEFAULT},
};
use utils::VecDisplay;
use view::{Tui, View};

use crate::tracing::Tracing;

use super::interface::{border_focus, border_unfocus, default_state, draw_scrollbar};

pub struct TaskStatus {
    pub solver:     Solver,
    pub is_solved:  bool,
    pub scroll_pos: ListState,
}

pub struct Tasks {
    tasks_list: Vec<Tracing>,

    tasks_pos:    ListState,
    focused_pane: usize,
}

impl Tasks {
    #[inline]
    pub fn new(rules: Arc<RulesEngine>, arg: impl IntoIterator<Item = Task>) -> Self {
        Self {
            tasks_list:   arg
                .into_iter()
                .map(|x| {
                    Tracing::new(TaskStatus {
                        solver:     Solver::new(
                            x,
                            rules.clone(),
                            DumperConfig {
                                sink:         "profiler".into(),
                                filename:     None,
                                use_profiler: true,
                            }
                            .build(),
                            EXECUTION_DEADLINE_DEFAULT,
                            Default::default(),
                        ),
                        is_solved:  false,
                        scroll_pos: default_state(),
                    })
                })
                .collect(),
            tasks_pos:    default_state(),
            focused_pane: 0,
        }
    }

    #[inline]
    pub fn solve(&mut self) {
        let tracing = self
            .tasks_list
            .get_mut(self.tasks_pos.selected().unwrap())
            .unwrap();
        let _ = tracing.task.solver.solve();
        tracing.task.is_solved = true;
    }

    #[inline]
    pub fn solve_all(&mut self) {
        for tracing in self.tasks_list.iter_mut() {
            if !tracing.task.is_solved {
                let _ = tracing.task.solver.solve();
                tracing.task.is_solved = true;
            }
        }
    }

    #[inline]
    pub fn select_next(&mut self) {
        if self.focused_pane == 0 {
            self.tasks_pos.select_next()
        } else {
            self.tasks_list
                .get_mut(self.tasks_pos.selected().unwrap())
                .unwrap()
                .task
                .scroll_pos
                .select_next()
        }
    }

    #[inline]
    pub fn select_previous(&mut self) {
        if self.focused_pane == 0 {
            self.tasks_pos.select_previous()
        } else {
            self.tasks_list
                .get_mut(self.tasks_pos.selected().unwrap())
                .unwrap()
                .task
                .scroll_pos
                .select_previous()
        }
    }

    #[inline]
    pub fn left(&mut self) {
        self.focused_pane = 0;
    }

    #[inline]
    pub fn right(&mut self) {
        self.focused_pane = 1;
    }

    #[inline]
    pub fn tracing(&mut self) -> &mut Tracing {
        self.tasks_list
            .get_mut(self.tasks_pos.selected().unwrap())
            .unwrap()
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.draw_tasks_list(frame, layout[0]);

        self.draw_solution(frame, layout[1]);
    }

    fn draw_tasks_list(&mut self, frame: &mut Frame, area: Rect) {
        let task_list = List::new(self.tasks_list.iter().map(|x| {
            let task_line_style = if !x.task.is_solved {
                Style::new()
            } else if x.task.solver.answer.is_none() {
                Style::new().fg(Color::Yellow).bold()
            } else if !x.task.solver.validate_answer() {
                Style::new().fg(Color::Red).bold()
            } else {
                Style::new().fg(Color::Green).bold()
            };

            Line::from(vec![
                Span::styled(x.task.solver.task.purpose.to_string(), task_line_style),
                Span::styled(
                    VecDisplay(&x.task.solver.task.conditions).to_string(),
                    task_line_style,
                ),
            ])
        }))
        .highlight_symbol(">")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.pane_style(0))
                .title("Tasks"),
        );

        frame.render_stateful_widget(task_list, area, &mut self.tasks_pos);
        draw_scrollbar(
            frame,
            area,
            self.tasks_list.len(),
            self.tasks_pos.selected().unwrap(),
        );
    }

    fn draw_solution(&mut self, frame: &mut Frame, area: Rect) {
        let pane_style = self.pane_style(1);
        let tracing = self
            .tasks_list
            .get_mut(self.tasks_pos.selected().unwrap())
            .unwrap();
        let mut renderer = Tui::default();
        View::try_from(&tracing.task.solver)
            .unwrap()
            .display_impl(&mut renderer)
            .unwrap();

        let solution_lines: Vec<_> = if tracing.task.is_solved {
            format!("Conditions:\n{}\n\nSolution", tracing.task.solver.task)
                .split('\n')
                .map(|x| Line::from(Span::from(x.to_owned())))
                .chain(renderer.output)
                .collect()
        } else {
            vec![Line::from(Span::from("Press s to solve".to_owned()))]
        };

        frame.render_stateful_widget(
            List::new(solution_lines.iter().cloned())
                .highlight_style(Style::new().underlined())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(pane_style)
                        .title("Detailed"),
                ),
            area,
            &mut tracing.task.scroll_pos,
        );

        draw_scrollbar(
            frame,
            area,
            solution_lines.len(),
            tracing.task.scroll_pos.selected().unwrap(),
        );
    }

    fn pane_style(&self, pane: usize) -> Style {
        if self.focused_pane == pane {
            border_focus()
        } else {
            border_unfocus()
        }
    }
}

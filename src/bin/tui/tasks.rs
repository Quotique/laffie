use std::sync::Arc;

use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{Block, Borders, List, ListState},
};

use mcore::{
    rule::RulesEngine,
    task::{DumperConfig, Solution, Task, EXECUTION_DEADLINE_DEFAULT},
};
use utils::VecDisplay;
use view::View;

use crate::tracing::Tracing;

use super::interface::{border_focus, border_unfocus, default_state, draw_scrollbar};

pub struct TaskStatus {
    pub solution: Solution,
    pub solved:   bool,
    pub scroll:   ListState,
}

pub struct Tasks {
    engine: Vec<Tracing>,

    list_state:   ListState,
    focused_pane: usize,
}

impl Tasks {
    #[inline]
    pub fn new(rules: Arc<RulesEngine>, arg: impl IntoIterator<Item = Task>) -> Self {
        Self {
            engine:       arg
                .into_iter()
                .map(|x| {
                    Tracing::new(TaskStatus {
                        solution: Solution::new(
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
                        solved:   false,
                        scroll:   default_state(),
                    })
                })
                .collect(),
            list_state:   default_state(),
            focused_pane: 0,
        }
    }

    #[inline]
    pub fn solve(&mut self) {
        let tracing = self
            .engine
            .get_mut(self.list_state.selected().unwrap())
            .unwrap();
        let _ = tracing.task.solution.solve();
        tracing.task.solved = true;
    }

    #[inline]
    pub fn select_next(&mut self) {
        if self.focused_pane == 0 {
            self.list_state.select_next()
        } else {
            self.engine
                .get_mut(self.list_state.selected().unwrap())
                .unwrap()
                .task
                .scroll
                .select_next()
        }
    }

    #[inline]
    pub fn select_previous(&mut self) {
        if self.focused_pane == 0 {
            self.list_state.select_previous()
        } else {
            self.engine
                .get_mut(self.list_state.selected().unwrap())
                .unwrap()
                .task
                .scroll
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
        self.engine
            .get_mut(self.list_state.selected().unwrap())
            .unwrap()
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let list = List::new(self.engine.iter().map(|x| {
            format!(
                "{} {}",
                x.task.solution.task.purpose,
                VecDisplay(&x.task.solution.task.conditions)
            )
        }))
        .highlight_symbol("> ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.pane_style(0))
                .title("Tasks"),
        );

        frame.render_stateful_widget(list, layout[0], &mut self.list_state);
        draw_scrollbar(
            frame,
            layout[0],
            self.engine.len(),
            self.list_state.selected().unwrap(),
        );

        let pane_style = self.pane_style(1);
        let tracing = self
            .engine
            .get_mut(self.list_state.selected().unwrap())
            .unwrap();
        let solution = if tracing.task.solved {
            format!(
                "Solution\n{}",
                View::try_from(&tracing.task.solution).unwrap()
            )
        } else {
            "Press s to solve".to_owned()
        };

        let solution_lines: Vec<String> = format!(
            "Conditions:\n{}\n\n{}",
            tracing.task.solution.task, solution
        )
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
            &mut tracing.task.scroll,
        );
        draw_scrollbar(
            frame,
            layout[1],
            solution_lines.len(),
            tracing.task.scroll.selected().unwrap(),
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

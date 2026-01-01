use std::hash;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, StatefulWidget, Widget},
};
use tui_tree_widget::{Tree, TreeItem};

use crate::{state::TaskState, theme::Theme};
use solver::task::{SharedSolution, Solution, TermInference, TermProps};

#[derive(Clone, Debug)]
pub struct TermId {
    pub solution: SharedSolution,
    pub idx:      usize,
}

#[derive(Default)]
pub struct TracingNavigation {}

impl TermId {
    pub fn new_solution(solution: SharedSolution) -> Self {
        Self { solution, idx: 0 }
    }

    pub fn new_term(solution: SharedSolution, idx: usize) -> Self {
        Self {
            solution,
            idx: idx + 1,
        }
    }
}

impl TracingNavigation {
    fn tree(&self, solution: SharedSolution) -> Vec<TreeItem<'static, TermId>> {
        solution
            .terms
            .iter()
            .enumerate()
            .map(|(num, x)| {
                let term_id = TermId::new_term(solution.clone(), num);
                let term_line = self.term_line(x, solution.cycles());

                let term_cycles: usize = x.inference.requirements().map(|x| x.cycles()).sum();
                let children: Vec<TreeItem<'static, TermId>> = match &x.inference {
                    TermInference::Rule { requirements, .. } => requirements
                        .iter()
                        .map(|x| {
                            TreeItem::new_leaf(
                                TermId::new_solution(x.clone()),
                                self.subtask_line(x.clone(), term_cycles),
                            )
                        })
                        .collect(),
                    TermInference::Transform { solution, .. } => {
                        vec![TreeItem::new_leaf(
                            TermId::new_solution(solution.clone()),
                            self.subtask_line(solution.clone(), term_cycles),
                        )]
                    }
                    _ => vec![],
                };
                if children.is_empty() {
                    TreeItem::new_leaf(term_id, term_line)
                } else {
                    TreeItem::new(term_id, term_line, children).unwrap()
                }
            })
            .collect()
    }

    fn term_line(&self, term: &TermProps, total_cycles: usize) -> Line<'static> {
        let style = if term.inference.is_proven() {
            self.theme().default_tree_item()
        } else {
            self.theme().unproven_tree_item()
        };

        let cycles: usize = term.inference.requirements().map(|x| x.cycles()).sum();

        Line::from(vec![
            Span::styled(term.term.to_string(), style),
            Span::from(format!(" {} {}", cycles, total_cycles)),
        ])
    }

    fn subtask_line(&self, solution: SharedSolution, total_cycles: usize) -> Line<'static> {
        let style = if solution.answer().is_some() {
            self.theme().default_tree_item()
        } else {
            self.theme().unproven_tree_item()
        };
        Line::from(vec![
            Span::styled(solution.task.goal.to_string(), style),
            Span::from(format!(" {} {}", solution.cycles(), total_cycles)),
        ])
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

impl StatefulWidget for &TracingNavigation {
    type State = TaskState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let panes = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        let path = Line::from(vec![Span::styled(
            state
                .tracing_state
                .iter()
                .map(|x| format!("[ {} ]", x.0.task.goal.to_string()))
                .collect::<Vec<String>>()
                .join(""),
            Style::default().add_modifier(Modifier::BOLD),
        )]);
        let path = Paragraph::new(path);
        <Paragraph as Widget>::render(path, panes[0], buf);

        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(panes[1]);

        let mut it = state.tracing_state.iter_mut().rev();
        for i in [1, 0] {
            if let Some(state) = it.next() {
                let items = self.tree(state.0.clone());
                let widget = Tree::new(&items)
                    .expect("all item identifiers are unique")
                    .experimental_scrollbar(Some(
                        Scrollbar::new(ScrollbarOrientation::VerticalRight)
                            .begin_symbol(None)
                            .track_symbol(None)
                            .end_symbol(None),
                    ))
                    .highlight_style(self.theme().tree_cursor_style())
                    .highlight_symbol("> ")
                    .block(Block::default().borders(if i == 1 {
                        Borders::LEFT
                    } else {
                        Borders::RIGHT
                    }));
                <Tree<TermId> as StatefulWidget>::render(widget, panes[i], buf, &mut state.1);
                if state.1.selected().is_empty() {
                    state.1.select_first();
                }
            }
        }
    }
}

impl Eq for TermId {}
impl PartialEq for TermId {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.solution.as_ref(), other.solution.as_ref()) && self.idx == other.idx
    }
}

impl hash::Hash for TermId {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        let ptr: *const Solution = self.solution.as_ref();
        ptr.hash(state);
        self.idx.hash(state);
    }
}

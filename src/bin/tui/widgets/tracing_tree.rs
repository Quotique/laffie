use std::hash;

use ratatui::{
    prelude::*,
    widgets::{Block, Scrollbar, ScrollbarOrientation, StatefulWidget},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use solver::task::{SharedSolution, Solution, TermInference};

use crate::theme::Theme;

#[derive(Clone, Debug)]
pub struct TermId {
    pub solution: SharedSolution,
    pub idx:      usize,
}

#[derive(Clone, Debug)]
pub struct TracingTree {
    pub solution: SharedSolution,
}

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

impl StatefulWidget for TracingTree {
    type State = TreeState<TermId>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let solution = self.solution.clone();

        let items = [self.tree(solution)];
        let widget = Tree::new(&items)
            .expect("all item identifiers are unique")
            .block(Block::bordered().title("Tracing"))
            .experimental_scrollbar(Some(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .track_symbol(None)
                    .end_symbol(None),
            ))
            .highlight_style(self.theme().tree_cursor_style())
            .highlight_symbol(">");
        <Tree<TermId> as StatefulWidget>::render(widget, area, buf, state);
    }
}

impl TracingTree {
    fn tree(&self, solution: SharedSolution) -> TreeItem<'static, TermId> {
        let children: Vec<_> = solution
            .terms
            .iter()
            .enumerate()
            .filter_map(|(num, x)| {
                let children = match &x.inference {
                    TermInference::Rule { requirements, .. } => {
                        requirements.iter().map(|x| self.tree(x.clone())).collect()
                    }
                    TermInference::Transform { solution, .. } => {
                        vec![self.tree(solution.clone())]
                    }
                    _ => {
                        return None;
                    }
                };
                Some((num, children))
            })
            .map(|(num, children)| {
                let term_id = TermId::new_term(solution.clone(), num);
                let line = self.tree_line(&term_id);
                if children.is_empty() {
                    TreeItem::new_leaf(term_id, line)
                } else {
                    TreeItem::new(term_id, line, children).unwrap()
                }
            })
            .collect();

        let term_id = TermId::new_solution(solution.clone());

        if children.is_empty() {
            TreeItem::new_leaf(term_id, solution.task.purpose.to_string())
        } else {
            TreeItem::new(term_id, solution.task.purpose.to_string(), children)
                .expect("index must be unique")
        }
    }

    // TODO: total cycles
    fn tree_line(&self, id: &TermId) -> Line<'static> {
        if id.idx == 0 {
            let style = if id.solution.answer().is_some() {
                self.theme().default_tree_item()
            } else {
                self.theme().unproven_tree_item()
            };
            Line::from(vec![
                Span::styled(id.solution.task.purpose.to_string(), style),
                Span::from(format!(" {} {}", id.solution.cycles(), 0)),
            ])
        } else {
            let term = &id.solution.terms[id.idx - 1];
            let style = if term.inference.is_proven() {
                self.theme().default_tree_item()
            } else {
                self.theme().unproven_tree_item()
            };

            Line::from(vec![
                Span::styled(term.term.to_string(), style),
                Span::from(format!(" {} {}", id.solution.cycles(), 0)),
            ])
        }
    }

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
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

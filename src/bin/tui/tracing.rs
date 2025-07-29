use std::{fmt::Display, hash};

use itertools::{chain, Itertools};
use ratatui::{
    prelude::*,
    widgets::{Block, List, Scrollbar, ScrollbarOrientation},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use solver::task::{SharedSolution, Solution, SolutionStatus, TermInference};

use super::{
    state::Command,
    theme::{draw_scrollbar, Theme},
};
use crate::tasks::TaskStatus;

#[derive(Clone, Debug)]
struct TermId {
    solution: SharedSolution,
    idx:      usize,
}

pub struct Tracing {
    pub task: TaskStatus,

    tree_state:   TreeState<TermId>,
    focused_pane: usize,
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

impl Tracing {
    pub fn new(task: TaskStatus) -> Self {
        Self {
            task,
            tree_state: Default::default(),
            focused_pane: 0,
        }
    }

    pub fn process(&mut self, command: Command) {
        let _ = match command {
            Command::Down => self.tree_state.key_down(),
            Command::Up => self.tree_state.key_up(),
            Command::Left => self.tree_state.key_left(),
            Command::Right => self.tree_state.key_right(),
            Command::Toggle => self.tree_state.toggle_selected(),
            _ => false,
        };
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.draw_profiler_tree(frame, layout[0]);
        self.draw_profiler_node_details(frame, layout[1]);
    }

    fn draw_profiler_tree(&mut self, frame: &mut Frame, area: Rect) {
        let solution = self.task.solution.clone();

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
        frame.render_stateful_widget(widget, area, &mut self.tree_state);
    }

    fn draw_profiler_node_details(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(selected) = self.tree_state.selected().last() {
            let text = if selected.idx == 0 {
                self.task_lines(selected.solution.as_ref())
            } else {
                self.term_inference_lines(selected)
            };
            frame.render_widget(
                List::new(text.iter().cloned())
                    .highlight_style(self.theme().list_cursor_style())
                    .block(self.theme().block(self.focused_pane == 1, "Detailed")),
                area,
            );
            let cursor_pos = self.task.scroll_pos.selected().unwrap();
            draw_scrollbar(frame, area, text.len(), cursor_pos);
        }
    }

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

    fn task_lines(&self, solution: &Solution) -> Vec<Line> {
        vec![
            self.key_value_line("Task: ", &solution.purpose),
            Line::default(),
            self.key_value_line(
                "Answer: ",
                solution
                    .answer()
                    .map(|x| Span::from(x.to_string()))
                    .unwrap_or(Span::styled("no answer", self.theme().error())),
            ),
            Line::default(),
            self.key_value_line("Cycles: ", solution.cycles()),
        ]
    }

    fn key_value_line<'a>(&self, key: &'a str, text: impl Display) -> Line<'a> {
        Line::from(vec![
            Span::styled(key, self.theme().highlighted()),
            Span::from(text.to_string()),
        ])
    }

    fn params_line(&self, key: impl Display, text: impl Display) -> Line<'static> {
        Line::from(vec![Span::raw(format!("  {key} = {text}"))])
    }

    fn term_inference_lines(&self, term_id: &TermId) -> Vec<Line<'static>> {
        let term = &term_id.solution.terms[term_id.idx - 1];
        match &term.inference {
            TermInference::Rule {
                parent,
                params,
                rule,
                requirements,
            } => {
                let parent = &term_id.solution.terms[*parent];
                let mut result = vec![
                    self.key_value_line("Parent: ", parent),
                    self.key_value_line("Term: ", term),
                    Line::default(),
                    self.key_value_line("Rule: ", rule),
                    self.key_value_line("Params:", ""),
                ];
                result.append(
                    &mut chain(
                        params.params.iter().map(|(k, v)| self.params_line(k, v)),
                        { params.arglists.iter() }
                            .map(|(k, v)| self.params_line(k, v.iter().format(", "))),
                    )
                    .collect(),
                );
                result.push(Line::default());
                result.push(self.key_value_line("Requirements:", ""));

                let proven = self.theme().proven_requirement();
                let unproven = self.theme().unproven_requirement();
                let skipped = self.theme().skipped_requirement();
                for i in requirements {
                    let purpose = &i.task.purpose.term;
                    result.push(Line::from(match i.status {
                        SolutionStatus::Answer(_) => {
                            Span::styled(format!("  ☑  {purpose}"), proven)
                        }
                        SolutionStatus::Err(_) => Span::styled(format!("  ☒  {purpose}"), unproven),
                        SolutionStatus::NotDone => Span::styled(format!("  ☐  {purpose}"), skipped),
                    }));
                }
                result.push(Line::default());
                result.push(self.key_value_line("Cycles: ", 0));
                result
            }

            _ => {
                vec![self.key_value_line("Term: ", term)]
            }
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

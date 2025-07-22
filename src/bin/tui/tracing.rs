use std::{fmt::Display, hash};

use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{Block, Borders, List, Scrollbar, ScrollbarOrientation},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use solver::{
    rule::SharedRule,
    task::{SharedSolution, Solution, TermInference},
    term::Term,
};

use super::interface::{border_focus, border_unfocus, draw_scrollbar};
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

impl Tracing {
    pub fn new(task: TaskStatus) -> Self {
        Self {
            task,
            tree_state: Default::default(),
            focused_pane: 0,
        }
    }

    #[inline]
    pub fn select_next(&mut self) {
        self.tree_state.key_down();
    }

    #[inline]
    pub fn select_previous(&mut self) {
        self.tree_state.key_up();
    }

    #[inline]
    pub fn left(&mut self) {
        self.tree_state.key_left();
    }

    #[inline]
    pub fn right(&mut self) {
        self.tree_state.key_right();
    }

    #[inline]
    pub fn toggle(&mut self) {
        self.tree_state.toggle_selected();
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
        let Some(solution) = &self.task.solution else {
            // unimplemented!("TODO: not solved task");
            return;
        };

        let items = [Self::tree(solution.clone())];
        let widget = Tree::new(&items)
            .expect("all item identifiers are unique")
            .block(
                Block::bordered()
                    .title("Tracing")
                    //.title_bottom(format!("{:?}", &mut self.tree_state)),
            )
            .experimental_scrollbar(Some(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .track_symbol(None)
                    .end_symbol(None),
            ))
            .highlight_style(
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">");
        frame.render_stateful_widget(widget, area, &mut self.tree_state);
    }

    fn draw_profiler_node_details(&mut self, frame: &mut Frame, area: Rect) {
        let pane_style = self.pane_style(1);

        if let Some(selected) = self.tree_state.selected().last() {
            let text = if selected.idx == 0 {
                let solution = &selected.solution;
                Self::task_lines(solution.as_ref())
            } else {
                let term = &selected.solution.terms[selected.idx - 1];
                if let TermInference::Rule {
                    parent,
                    rule,
                    requirements,
                } = &term.inference
                {
                    Self::term_inference_lines(
                        &term.term,
                        &selected.solution.terms[*parent].term,
                        rule.clone(),
                        requirements,
                    )
                } else {
                    Self::term_lines(&term.term)
                }
            };
            frame.render_widget(
                List::new(text.iter().cloned())
                    .highlight_style(Style::new().underlined())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(pane_style)
                            .title("Detailed"),
                    ),
                area,
            );
            draw_scrollbar(
                frame,
                area,
                text.len(),
                // task.scroll.selected().unwrap()
                0,
            );
        }
    }

    fn tree(solution: SharedSolution) -> TreeItem<'static, TermId> {
        let children: Vec<_> = solution
            .terms
            .iter()
            .enumerate()
            .filter_map(|(num, x)| {
                let children = match &x.inference {
                    TermInference::Rule { requirements, .. } => {
                        requirements.iter().map(|x| Self::tree(x.clone())).collect()
                    }
                    TermInference::Transform { solution, .. } => {
                        vec![Self::tree(solution.clone())]
                    }
                    _ => {
                        return None;
                    }
                };
                Some((num, children))
            })
            .map(|(num, children)| {
                let term_id = TermId {
                    solution: solution.clone(),
                    idx:      num + 1,
                };
                let line = Self::tree_line(&term_id);
                if children.is_empty() {
                    TreeItem::new_leaf(term_id, line)
                } else {
                    TreeItem::new(term_id, line, children).unwrap()
                }
            })
            .collect();

        let term_id = TermId {
            solution: solution.clone(),
            idx:      0,
        };

        if children.is_empty() {
            TreeItem::new_leaf(term_id, solution.task.purpose.to_string())
        } else {
            TreeItem::new(term_id, solution.task.purpose.to_string(), children)
                .expect("index must be unique")
        }
    }

    // TODO: total cycles
    fn tree_line(id: &TermId) -> Line<'static> {
        let default_style = Style::new();
        let not_proved_style = Style::new().crossed_out().dim();

        if id.idx == 0 {
            let style = if id.solution.answer().is_some() {
                default_style
            } else {
                not_proved_style
            };
            Line::from(vec![
                Span::styled(id.solution.task.purpose.to_string(), style),
                Span::from(format!(" {} {}", id.solution.cycles(), 0)),
            ])
        } else {
            let term = &id.solution.terms[id.idx - 1];
            let style = if term.inference.is_proven() {
                default_style
            } else {
                not_proved_style
            };

            Line::from(vec![
                Span::styled(term.term.to_string(), style),
                Span::from(format!(" {} {}", id.solution.cycles(), 0)),
            ])
        }
    }

    fn task_lines(solution: &Solution) -> Vec<Line> {
        let highlighted = Style::new().fg(Color::LightBlue).bold();
        let no_answer = Style::new().fg(Color::Red).bold();

        vec![
            Line::from(vec![
                Span::styled("Task: ", highlighted),
                Span::from(solution.purpose.to_string()),
            ]),
            Line::default(),
            Line::from(vec![
                Span::styled("Answer: ", highlighted),
                if let Some(answer) = solution.answer() {
                    Span::from(answer.to_string())
                } else {
                    Span::styled("no answer".to_owned(), no_answer)
                },
            ]),
            Line::default(),
            Line::from(vec![
                Span::styled("Cycles: ", highlighted),
                Span::from(solution.cycles().to_string()),
            ]),
        ]
    }

    fn key_value_line<'a>(key: &'a str, text: impl Display) -> Line<'a> {
        let highlighted = Style::new().fg(Color::LightBlue).bold();
        Line::from(vec![
            Span::styled(key, highlighted),
            Span::from(text.to_string()),
        ])
    }

    fn term_inference_lines(
        term: &Term,
        parent: &Term,
        rule: SharedRule,
        requirements: &Vec<SharedSolution>,
    ) -> Vec<Line<'static>> {
        let highlighted = Style::new().fg(Color::LightBlue).bold();
        let mut result = vec![
            Self::key_value_line("Parent: ", parent),
            Self::key_value_line("Term: ", term),
            Line::default(),
            Self::key_value_line("Rule: ", rule),
            Line::from(Span::styled("Params:", highlighted)),
        ];
        // TODO:
        // result.append(
        //     &mut term
        //         .params
        //         .iter()
        //         .map(|(param, value)| Line::from(vec![Span::raw(format!("  {param} =
        // {value}"))]))         .collect(),
        // );
        result.push(Line::default());
        result.push(Self::key_value_line("Requirements:", ""));

        let proven = Style::new().fg(Color::Green).bold();
        let unproven = Style::new().fg(Color::Red).bold();
        // let skiped = Style::new().fg(Color::Gray).bold();
        for i in requirements {
            let span = match i.answer().is_some() {
                true => Span::styled(format!("  ☑  {}", i.task.purpose.term), proven),
                false => Span::styled(format!("  ☒  {}", i.task.purpose.term), unproven),
                // None => Span::styled(format!("  ☐  {}", i.0), skiped),
            };
            result.push(Line::from(span));
        }
        result.push(Line::default());
        result.push(Self::key_value_line("Cycles: ", 0));
        result
    }

    fn term_lines(term: &Term) -> Vec<Line> {
        vec![Self::key_value_line("Term: ", term)]
    }

    fn _draw_solution_text(solution: &Solution, pane_style: Style, frame: &mut Frame, area: Rect) {
        let empty = vec![];
        let solution: Vec<_> = solution
            .terms
            .iter()
            .flat_map(|x| {
                std::iter::once(x.to_string()).chain(
                    x.inference
                        .requirements()
                        .unwrap_or(&empty)
                        .iter()
                        .map(|x| format!("  {}", x.task.purpose.term)),
                )
            })
            .collect();
        frame.render_widget(
            List::new(solution.iter().cloned())
                .highlight_style(Style::new().underlined())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(pane_style)
                        .title("Detailed"),
                ),
            area,
        );
        draw_scrollbar(
            frame,
            area,
            solution.len(),
            // task.scroll.selected().unwrap()
            0,
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

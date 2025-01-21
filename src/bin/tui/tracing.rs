use ego_tree::{NodeId, NodeRef};
use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{Block, Borders, List, Scrollbar, ScrollbarOrientation},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use mcore::task::{ProfilerNode, Solution};

use super::interface::{border_focus, border_unfocus, draw_scrollbar};
use crate::tasks::TaskStatus;

pub struct Tracing {
    pub task: TaskStatus,

    tree_state:   TreeState<Option<NodeId>>,
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

    fn tree(
        profiler: &NodeRef<ProfilerNode>,
        total_cycles: usize,
    ) -> TreeItem<'static, Option<NodeId>> {
        let default_style = Style::new();
        let not_proved_style = Style::new().crossed_out().dim();

        let text = match profiler.value() {
            ProfilerNode::Helper(task) => Line::from(vec![
                Span::from(task.purpose.clone()),
                Span::from(format!(" {} {}", profiler.value().cycles(), total_cycles)),
            ]),
            ProfilerNode::Suppose(suppose) => Line::from(vec![
                Span::styled(
                    suppose.term.clone(),
                    if suppose.result {
                        default_style
                    } else {
                        not_proved_style
                    },
                ),
                Span::from(format!(" {} {}", profiler.value().cycles(), total_cycles)),
            ]),
        };

        if profiler.has_children() {
            TreeItem::new(
                Some(profiler.id()),
                text,
                profiler
                    .children()
                    .map(|s| Self::tree(&s, profiler.value().cycles()))
                    .collect(),
            )
            .unwrap()
        } else {
            TreeItem::new_leaf(Some(profiler.id()), text)
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.draw_profiler_tree(frame, layout[0]);
        self.draw_profiler_node_details(frame, layout[1]);
    }

    fn draw_profiler_node_details(&mut self, frame: &mut Frame, area: Rect) {
        let pane_style = self.pane_style(1);
        let highlighted = Style::new().fg(Color::LightBlue).bold();

        if let Some(selected) = self.tree_state.selected().last() {
            let node = self.task.solution.dumper.profiler().unwrap().lock();

            let text = match node.task.get(selected.unwrap()).unwrap().value() {
                ProfilerNode::Helper(task) => vec![
                    Line::from(vec![
                        Span::styled("Task: ", highlighted),
                        Span::from(&task.purpose),
                    ]),
                    Line::default(),
                    Line::from(vec![
                        Span::styled("Cycles: ", highlighted),
                        Span::from(task.cycles().to_string()),
                    ]),
                ],
                ProfilerNode::Suppose(suppose) => {
                    let mut result = vec![
                        Line::from(vec![
                            Span::styled("Term: ", highlighted),
                            Span::from(&suppose.term),
                        ]),
                        Line::default(),
                        Line::from(vec![
                            Span::styled("Rule: ", highlighted),
                            Span::from(&suppose.rule),
                        ]),
                        Line::default(),
                        Line::from(vec![
                            Span::styled("Cycles: ", highlighted),
                            Span::from(suppose.cycles().to_string()),
                        ]),
                        Line::default(),
                        Line::from(Span::styled("Requirements:", highlighted)),
                    ];
                    result.append(
                        &mut suppose
                            .requirements
                            .iter()
                            .map(|x| Line::from(vec![Span::raw("  "), Span::from(x)]))
                            .collect(),
                    );
                    result
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

    fn draw_profiler_tree(&mut self, frame: &mut Frame, area: Rect) {
        let items = [Self::tree(
            &self
                .task
                .solution
                .dumper
                .profiler()
                .unwrap()
                .lock()
                .task
                .root(),
            *self.task.solution.cycles.borrow(),
        )];
        let widget = Tree::new(&items)
            .expect("all item identifiers are unique")
            .block(
                Block::bordered()
                    .title("Tree Widget")
                    .title_bottom(format!("{:?}", &mut self.tree_state)),
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

    fn _draw_solution_text(solution: &Solution, pane_style: Style, frame: &mut Frame, area: Rect) {
        let solution: Vec<_> = solution
            .terms
            .iter()
            .flat_map(|x| {
                std::iter::once(x.to_string())
                    .chain(x.requirements.iter().map(|x| format!("  {}", x)))
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

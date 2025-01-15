use ego_tree::{NodeId, NodeRef};
use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{Block, Borders, List, Scrollbar, ScrollbarOrientation},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use mcore::{
    rule::Suppose,
    task::{ProfilerNode, Solution},
};

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
        self.focused_pane = 0;
    }

    #[inline]
    pub fn right(&mut self) {
        self.focused_pane = 1;
    }

    #[inline]
    pub fn toggle(&mut self) {
        self.tree_state.toggle_selected();
    }

    fn tree(profiler: &NodeRef<ProfilerNode>) -> TreeItem<'static, Option<NodeId>> {
        let text = match profiler.value() {
            ProfilerNode::Helper(task) => task.purpose.clone(),
            ProfilerNode::Suppose(suppose) => suppose.term.clone(),
        };

        if profiler.has_children() {
            TreeItem::new(
                Some(profiler.id()),
                text,
                profiler.children().map(|s| Self::tree(&s)).collect(),
            )
            .unwrap()
        } else {
            TreeItem::new_leaf(Some(profiler.id()), text)
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let pane_style = self.pane_style(0);

        // if !self.task.solved {
        //     todo!();
        //     // return self.draw_solution(frame, area);
        // }

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

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
            .highlight_symbol(">> ");
        frame.render_stateful_widget(widget, layout[0], &mut self.tree_state);

        if let Some(selected) = self.tree_state.selected().last() {
            let node = self.task.solution.dumper.profiler().unwrap().lock();

            let text1 = match node.task.get(selected.unwrap()).unwrap().value() {
                ProfilerNode::Helper(task) => task.purpose.clone(),
                ProfilerNode::Suppose(suppose) => suppose.term.clone(),
            };
            let text = [
                text1,
                "test1".to_owned(),
                "test2".to_owned(),
                format!("{:?}", self.tree_state.selected()),
            ];
            frame.render_widget(
                List::new(text.iter().cloned())
                    .highlight_style(Style::new().underlined())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(pane_style)
                            .title("Detailed"),
                    ),
                layout[1],
            );
            draw_scrollbar(
                frame,
                layout[1],
                text.len(),
                // task.scroll.selected().unwrap()
                0,
            );
        }
        // Self::draw_solution_text(&self.task.solution, pane_style, frame,
        // layout[0]); Self::draw_solution_text(
        //    &task.solution.terms.first().unwrap(),
        //    pane_style,
        //    frame,
        //    layout[1],
        //)
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

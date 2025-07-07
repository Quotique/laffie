use std::{rc::Rc, sync::Arc};

use ego_tree::{iter::Edge, NodeId, NodeMut, NodeRef, Tree};
use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{Block, Borders, List, ListState, Scrollbar, ScrollbarOrientation},
};
use tui_tree_widget::{Tree as TuiTree, TreeItem, TreeState};

use solver::{
    rule::RulesEngine,
    task::{DumperConfig, Solution, Solver, Task},
    CompactString,
};
use utils::VecDisplay;
use view::{Tui, View};

use crate::tracing::Tracing;

use super::interface::{border_focus, border_unfocus, default_state, draw_scrollbar};

pub struct TaskStatus {
    pub task:         Task,
    pub rules_engine: Arc<RulesEngine>,
    pub solution:     Option<Rc<Solution>>,
    pub scroll_pos:   ListState,
}

pub struct Tasks {
    tasks:            Vec<Tracing>,
    tasks_index:      Tree<TasksNode>,
    tasks_tree_state: TreeState<Option<NodeId>>,

    exec_deadline: usize,
    focused_pane:  usize,
}

enum TasksNode {
    Task(usize),
    Directory {
        dir_name:           CompactString,
        solved_count:       usize,
        unsolved_count:     usize,
        wrong_answer_count: usize,
        not_runned_count:   usize,
    },
}

impl TasksNode {
    pub fn new_task(task_pos: usize) -> Self {
        Self::Task(task_pos)
    }

    pub fn new_directory(dir_name: CompactString) -> Self {
        Self::Directory {
            dir_name,
            solved_count: 0,
            unsolved_count: 0,
            wrong_answer_count: 0,
            not_runned_count: 0,
        }
    }
}

impl Tasks {
    #[inline]
    pub fn new(
        exec_deadline: usize,
        rules: Arc<RulesEngine>,
        arg: impl IntoIterator<Item = Task>,
    ) -> Self {
        let mut result = Self {
            tasks: Default::default(),
            tasks_index: Tree::new(TasksNode::new_directory("Tasks".into())),
            tasks_tree_state: Default::default(),
            exec_deadline,
            focused_pane: 0,
        };

        for task in arg.into_iter() {
            result.add_task(rules.clone(), task);
        }

        result
    }

    pub fn replace_rules(&mut self, rules: Arc<RulesEngine>) {
        for task in self.tasks.iter_mut() {
            task.task.rules_engine = rules.clone();
        }
    }

    #[inline]
    pub fn solve_all(&mut self) {
        let ids: Vec<_> = self.tasks_index.nodes().map(|x| x.id()).collect();
        for node in ids {
            self.solve_node_id(node);
        }
    }

    #[inline]
    pub fn solve(&mut self) {
        if let Some(selected) = self.tasks_tree_state.selected().last() {
            self.solve_node_id(selected.unwrap());
        }
    }

    #[inline]
    pub fn select_next(&mut self) {
        if self.focused_pane == 0 {
            self.tasks_tree_state.key_down();
        } else if let Some(selected) = self.tasks_tree_state.selected().last() {
            if let TasksNode::Task(tracing) =
                self.tasks_index.get_mut(selected.unwrap()).unwrap().value()
            {
                self.tasks[*tracing].task.scroll_pos.select_next()
            }
        }
    }

    #[inline]
    pub fn select_previous(&mut self) {
        if self.focused_pane == 0 {
            self.tasks_tree_state.key_up();
        } else if let Some(selected) = self.tasks_tree_state.selected().last() {
            if let TasksNode::Task(tracing) =
                self.tasks_index.get_mut(selected.unwrap()).unwrap().value()
            {
                self.tasks[*tracing].task.scroll_pos.select_previous()
            }
        }
    }

    #[inline]
    pub fn toggle(&mut self) {
        if self.focused_pane == 0 {
            self.tasks_tree_state.toggle_selected();
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
    pub fn tracing(&mut self) -> Option<&mut Tracing> {
        if let Some(selected) = self.tasks_tree_state.selected().last() {
            if let TasksNode::Task(tracing) =
                self.tasks_index.get(selected.unwrap()).unwrap().value()
            {
                return self.tasks.get_mut(*tracing);
            }
        }
        None
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.draw_tasks_list(frame, layout[0]);
        self.draw_solution(frame, layout[1]);
    }

    fn tree(&self, tasks_node: &NodeRef<TasksNode>) -> TreeItem<'static, Option<NodeId>> {
        let text = match tasks_node.value() {
            TasksNode::Task(task) => {
                let task_line_style = if self.tasks[*task].task.solution.is_some() {
                    Style::new()
                } else if self.tasks[*task]
                    .task
                    .solution
                    .as_ref()
                    .unwrap()
                    .answer
                    .is_none()
                {
                    Style::new().fg(Color::Yellow).bold()
                } else if !self.tasks[*task]
                    .task
                    .solution
                    .as_ref()
                    .unwrap()
                    .validate_answer()
                {
                    Style::new().fg(Color::Red).bold()
                } else {
                    Style::new().fg(Color::Green).bold()
                };

                Line::from(vec![
                    Span::styled(
                        // TODO: unwrap bug
                        self.tasks[*task]
                            .task
                            .solution
                            .as_ref()
                            .unwrap()
                            .purpose
                            .to_string(),
                        task_line_style,
                    ),
                    Span::from(" "),
                    Span::styled(
                        VecDisplay(&self.tasks[*task].task.task.conditions).to_string(),
                        task_line_style,
                    ),
                ])
            }
            TasksNode::Directory {
                dir_name,
                solved_count,
                unsolved_count,
                wrong_answer_count,
                not_runned_count,
            } => Line::from(vec![
                Span::styled(dir_name.to_string(), Style::new().fg(Color::LightBlue)),
                Span::from(format!("[{not_runned_count}")),
                Span::styled(format!(" {solved_count}"), Style::new().fg(Color::Green)),
                Span::styled(format!(" {unsolved_count}"), Style::new().fg(Color::Yellow)),
                Span::styled(
                    format!(" {wrong_answer_count}"),
                    Style::new().fg(Color::Red),
                ),
                Span::from("]".to_owned()),
            ]),
        };

        if tasks_node.has_children() {
            TreeItem::new(
                Some(tasks_node.id()),
                text,
                tasks_node.children().map(|s| self.tree(&s)).collect(),
            )
            .unwrap()
        } else {
            TreeItem::new_leaf(Some(tasks_node.id()), text)
        }
    }

    fn draw_tasks_list(&mut self, frame: &mut Frame, area: Rect) {
        let items = [self.tree(&self.tasks_index.root())];

        let widget = TuiTree::new(&items)
            .expect("all item identifiers are unique")
            .block(
                Block::bordered()
                    .title("Tasks")
                    .border_style(self.pane_style(0))
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
        frame.render_stateful_widget(widget, area, &mut self.tasks_tree_state);
    }

    fn draw_solution(&mut self, frame: &mut Frame, area: Rect) {
        let pane_style = self.pane_style(1);

        let Some(selected) = self.tasks_tree_state.selected().last().cloned().flatten() else {
            return;
        };
        match self.tasks_index.get(selected).unwrap().value() {
            TasksNode::Task(task_id) => {
                let tracing = self.tasks.get_mut(*task_id).unwrap();
                let mut renderer = Tui::default();
                View::try_from(tracing.task.solution.as_ref().unwrap().as_ref())
                    .unwrap()
                    .display_impl(&mut renderer)
                    .unwrap();
                let mut lines: Vec<_> = format!("Task {}\n\nSolution", tracing.task.task)
                    .split('\n')
                    .map(|x| Line::from(Span::from(x.to_owned())))
                    .collect();
                if tracing.task.solution.is_some() {
                    lines.append(&mut renderer.output);
                } else {
                    lines.push(Line::from(Span::from("Press s to solve".to_owned())));
                };
                frame.render_stateful_widget(
                    List::new(lines.iter().cloned())
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
                    lines.len(),
                    tracing.task.scroll_pos.selected().unwrap(),
                );
            }
            TasksNode::Directory {
                dir_name,
                solved_count,
                unsolved_count,
                wrong_answer_count,
                not_runned_count,
            } => {
                let lines = [
                    Line::from(vec![
                        Span::styled("Group: ", Style::new().fg(Color::LightBlue)),
                        Span::styled(dir_name.to_string(), Style::new()),
                    ]),
                    Line::default(),
                    Line::from(vec![
                        Span::styled("Total: ", Style::new().fg(Color::LightBlue)),
                        Span::styled(
                            (solved_count + unsolved_count + wrong_answer_count + not_runned_count)
                                .to_string(),
                            Style::new(),
                        ),
                    ]),
                    Line::default(),
                    Line::from(vec![
                        Span::styled("Not runned: ", Style::new().fg(Color::LightBlue)),
                        Span::from(format!("{not_runned_count}")),
                    ]),
                    Line::from(vec![
                        Span::styled("Solved: ", Style::new().fg(Color::LightBlue)),
                        Span::styled(format!(" {solved_count}"), Style::new().fg(Color::Green)),
                    ]),
                    Line::from(vec![
                        Span::styled("Not solved: ", Style::new().fg(Color::LightBlue)),
                        Span::styled(format!(" {unsolved_count}"), Style::new().fg(Color::Yellow)),
                    ]),
                    Line::from(vec![
                        Span::styled("Wrong answers: ", Style::new().fg(Color::LightBlue)),
                        Span::styled(
                            format!(" {wrong_answer_count}"),
                            Style::new().fg(Color::Red),
                        ),
                    ]),
                ];

                frame.render_widget(
                    List::new(lines.iter().cloned())
                        .highlight_style(Style::new().underlined())
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(pane_style)
                                .title("Detailed"),
                        ),
                    area,
                );
            }
        };
    }

    fn add_task(&mut self, rules: Arc<RulesEngine>, task: Task) {
        self.tasks.push(Tracing::new(TaskStatus {
            task,
            rules_engine: rules,
            solution: None,
            scroll_pos: default_state(),
        }));

        let group = self.tasks.last().unwrap().task.task.group.clone();
        let index = self.tasks.len() - 1;
        let node_id = {
            let mut node = self.find_node(group.as_str());
            node.append(TasksNode::new_task(index));
            node.id()
        };
        self.counters_update(node_id, 0, 0, 0, 1);
    }

    fn find_node<'a>(&'a mut self, path: &str) -> NodeMut<'a, TasksNode> {
        let mut current_node_id = self.tasks_index.root().id();
        for i in path.split(['/']).filter(|x| !x.is_empty()) {
            if let Some(next_node) = self
                .tasks_index
                .get(current_node_id)
                .unwrap()
                .children()
                .find(|x| {
                    if let TasksNode::Directory { dir_name, .. } = x.value() {
                        dir_name == i
                    } else {
                        false
                    }
                })
            {
                current_node_id = next_node.id();
            } else {
                current_node_id = self
                    .tasks_index
                    .get_mut(current_node_id)
                    .unwrap()
                    .append(TasksNode::new_directory(i.into()))
                    .id();
            }
        }
        self.tasks_index.get_mut(current_node_id).unwrap()
    }

    fn counters_update(
        &mut self,
        node_id: NodeId,
        solved_delta: isize,
        unsolved_delta: isize,
        wrong_answer_delta: isize,
        not_runned_delta: isize,
    ) {
        let mut node_id = Some(node_id);

        while let Some(id) = node_id {
            let mut node = self.tasks_index.get_mut(id).unwrap();
            match node.value() {
                TasksNode::Directory {
                    solved_count,
                    unsolved_count,
                    wrong_answer_count,
                    not_runned_count,
                    ..
                } => {
                    *solved_count = (*solved_count as isize + solved_delta) as usize;
                    *unsolved_count = (*unsolved_count as isize + unsolved_delta) as usize;
                    *wrong_answer_count =
                        (*wrong_answer_count as isize + wrong_answer_delta) as usize;
                    *not_runned_count = (*not_runned_count as isize + not_runned_delta) as usize;
                }
                TasksNode::Task(_) => {}
            }
            node_id = node.parent().map(|x| x.id())
        }
    }

    fn solve_node_id(&mut self, node_id: NodeId) {
        let Some(TasksNode::Task(task_idx)) =
            self.tasks_index.get(node_id).map(|node| node.value())
        else {
            if let Some(node) = self.tasks_index.get(node_id) {
                let child_ids: Vec<_> = node
                    .traverse()
                    .filter_map(|x| {
                        let node = match x {
                            Edge::Open(node) => node,
                            Edge::Close(node) => node,
                        };
                        match node.value() {
                            TasksNode::Task(_) => Some(node.id()),
                            TasksNode::Directory { .. } => None,
                        }
                    })
                    .collect();
                for id in child_ids {
                    self.solve_node_id(id);
                }
            }
            return;
        };
        let mut solved_delta: isize = 0;
        let mut unsolved_delta: isize = 0;
        let mut wrong_answer_delta: isize = 0;
        let mut not_runned_delta: isize = 0;

        // Mark previous task status to remove
        if self.tasks[*task_idx].task.solution.is_none() {
            not_runned_delta -= 1;
        } else if self.tasks[*task_idx]
            .task
            .solution
            .as_ref()
            .unwrap()
            .answer
            .is_none()
        {
            unsolved_delta -= 1;
        } else if !self.tasks[*task_idx]
            .task
            .solution
            .as_ref()
            .unwrap()
            .validate_answer()
        {
            wrong_answer_delta -= 1;
        } else {
            solved_delta -= 1;
        };
        let task = &mut self.tasks[*task_idx].task;
        let mut solver = Solver::new(
            task.rules_engine.clone(),
            DumperConfig {
                sink:     "profiler".into(),
                filename: None,
            }
            .build(),
            self.exec_deadline,
        );
        task.solution = Some(match solver.solve(task.task.clone()) {
            Ok(solution) => solution,
            Err((solution, _)) => solution,
        });

        // Add new task status to remove
        if task.solution.as_ref().unwrap().answer.is_none() {
            unsolved_delta += 1;
        } else if !task.solution.as_ref().unwrap().validate_answer() {
            wrong_answer_delta += 1;
        } else {
            solved_delta += 1;
        };

        // Propagate changes to all parent nodes
        self.counters_update(
            node_id,
            solved_delta,
            unsolved_delta,
            wrong_answer_delta,
            not_runned_delta,
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

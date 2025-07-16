use std::sync::Arc;

use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{Block, Borders, List, ListState, Scrollbar, ScrollbarOrientation},
};
use trees::{tr, Node, Tree};
use tui_tree_widget::{Tree as TuiTree, TreeItem, TreeState};

use solver::{
    rule::RulesEngine,
    task::{DumperConfig, SharedSolution, Solver, Task},
    CompactString,
};
use utils::{IndexedTree, TreeIndex, VecDisplay};
use view::{Tui, View};

use crate::tracing::Tracing;

use super::interface::{border_focus, border_unfocus, default_state, draw_scrollbar};

pub struct TaskStatus {
    pub task:         Task,
    pub rules_engine: Arc<RulesEngine>,
    pub solution:     Option<SharedSolution>,
    pub scroll_pos:   ListState,
}

pub struct Tasks {
    tasks:            Vec<Tracing>,
    tasks_index:      Tree<TasksNode>,
    tasks_tree_state: TreeState<TreeIndex>,

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

    #[inline]
    pub fn replace_rules(&mut self, rules: Arc<RulesEngine>) {
        for task in self.tasks.iter_mut() {
            task.task.rules_engine = rules.clone();
        }
    }

    #[inline]
    pub fn solve_all(&mut self) {
        self.solve_node_id(&self.tasks_index.root().id());
    }

    #[inline]
    pub fn solve(&mut self) {
        if let Some(selected) = self.tasks_tree_state.selected().last().cloned() {
            self.solve_node_id(&selected);
        }
    }

    #[inline]
    pub fn select_next(&mut self) {
        if self.focused_pane == 0 {
            self.tasks_tree_state.key_down();
        } else if let Some(selected) = self.tasks_tree_state.selected().last() {
            if let TasksNode::Task(tracing) = self.tasks_index[selected].data() {
                self.tasks[*tracing].task.scroll_pos.select_next()
            }
        }
    }

    #[inline]
    pub fn select_previous(&mut self) {
        if self.focused_pane == 0 {
            self.tasks_tree_state.key_up();
        } else if let Some(selected) = self.tasks_tree_state.selected().last() {
            if let TasksNode::Task(tracing) = self.tasks_index[selected].data() {
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
            if let TasksNode::Task(tracing) = self.tasks_index[selected].data() {
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

    fn tree(&self, tasks_node: &Node<TasksNode>) -> TreeItem<'static, TreeIndex> {
        let text = match tasks_node.data() {
            TasksNode::Task(task) => {
                let task_line_style = if self.tasks[*task].task.solution.is_none() {
                    Style::new()
                } else if self.tasks[*task]
                    .task
                    .solution
                    .as_ref()
                    .unwrap()
                    .answer()
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
                        self.tasks[*task].task.task.purpose.to_string(),
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

        if tasks_node.degree() > 0 {
            TreeItem::new(
                tasks_node.id(),
                text,
                tasks_node.iter().map(|s| self.tree(s)).collect(),
            )
            .unwrap()
        } else {
            TreeItem::new_leaf(tasks_node.id(), text)
        }
    }

    fn draw_tasks_list(&mut self, frame: &mut Frame, area: Rect) {
        let items = [self.tree(self.tasks_index.root())];

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

        let Some(selected) = self.tasks_tree_state.selected().last().cloned() else {
            return;
        };
        match self.tasks_index[&selected].data() {
            TasksNode::Task(task_id) => {
                let tracing = self.tasks.get_mut(*task_id).unwrap();
                let mut lines: Vec<_> = format!("Task {}\n\nSolution", tracing.task.task)
                    .split('\n')
                    .map(|x| Line::from(Span::from(x.to_owned())))
                    .collect();

                if tracing.task.solution.is_some() {
                    let mut renderer = Tui::default();
                    View::try_from(tracing.task.solution.as_ref().unwrap().as_ref())
                        .unwrap()
                        .display_impl(&mut renderer)
                        .unwrap();
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
            let node = self.find_node_mut(group.as_str());
            node.push_back(tr(TasksNode::new_task(index)));
            node.id()
        };
        self.counters_update(&node_id, 0, 0, 0, 1);
    }

    fn find_node_mut<'a>(&'a mut self, path: &str) -> &'a mut Node<TasksNode> {
        let mut current_node = self.tasks_index.root_mut().get_mut();
        for i in path.split(['/']).filter(|x| !x.is_empty()) {
            let next_idx = if let Some(next_idx) = current_node
                .iter_mut()
                .enumerate()
                .find(|(_, x)| {
                    if let TasksNode::Directory { dir_name, .. } = x.data() {
                        dir_name == i
                    } else {
                        false
                    }
                })
                .map(|(num, _)| num)
            {
                next_idx
            } else {
                current_node.push_back(tr(TasksNode::new_directory(i.into())));
                current_node.degree() - 1
            };
            current_node = current_node.iter_mut().nth(next_idx).unwrap().get_mut();
        }
        current_node
    }

    fn counters_update(
        &mut self,
        node_id: &TreeIndex,
        solved_delta: isize,
        unsolved_delta: isize,
        wrong_answer_delta: isize,
        not_runned_delta: isize,
    ) {
        let mut node = self.tasks_index.get_mut(node_id);
        while let Some(n) = node {
            match n.data_mut() {
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

            // TODO: optimize
            let parent_id = n.parent().map(|x| x.id());
            node = parent_id.and_then(|id| self.tasks_index.get_mut(&id));
        }
    }

    fn solve_node_id(&mut self, node_id: &TreeIndex) {
        let Some(node) = self.tasks_index.get_mut(node_id) else {
            return;
        };

        let task_idx = match node.data() {
            TasksNode::Directory { .. } => {
                let indexes: Vec<_> = node.iter().map(|x| x.id()).collect();
                for id in indexes {
                    self.solve_node_id(&id);
                }
                return;
            }
            TasksNode::Task(idx) => idx,
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
            .answer()
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
        task.solution = Some(solver.solve(task.task.clone()));

        // Add new task status to remove
        if task.solution.as_ref().unwrap().answer().is_none() {
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

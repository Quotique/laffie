use std::{fmt::Display, sync::Arc};

use itertools::Itertools;
use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{block::Title, Block, Borders, List, ListState, Scrollbar, ScrollbarOrientation},
};
use trees::{tr, Node, Tree};
use tui_tree_widget::{Tree as TuiTree, TreeItem, TreeState};

use solver::{
    rule::RulesEngine,
    task::{DumperConfig, SharedSolution, Solution, SolutionStatus, Solver, StepsSource, Task},
    CompactString,
};
use utils::{IndexedTree, TreeIndex};

use crate::tracing::Tracing;

use super::state::{border_focus, border_unfocus, default_state, draw_scrollbar};

pub struct TaskStatus {
    pub task:         Task,
    pub rules_engine: Arc<RulesEngine>,
    pub solution:     SharedSolution,
    pub scroll_pos:   ListState,
}

pub struct Tasks {
    tasks:            Vec<Tracing>,
    tasks_index:      Tree<TasksNode>,
    tasks_tree_state: TreeState<TreeIndex>,

    exec_deadline: usize,
    focused_pane:  usize,
}

pub struct Theme {}

impl Theme {
    pub fn tree_cursor_style(&self) -> Style {
        Style::new()
            .fg(Color::Black)
            .bg(Color::LightGreen)
            .add_modifier(Modifier::BOLD)
    }

    pub fn list_cursor_style(&self) -> Style {
        Style::new().underlined()
    }

    pub fn wrong_answer(&self) -> Style {
        Style::new().fg(Color::Red)
    }

    pub fn unsolved(&self) -> Style {
        Style::new().fg(Color::Yellow)
    }

    pub fn solved(&self) -> Style {
        Style::new().fg(Color::Green)
    }

    pub fn not_started(&self) -> Style {
        Style::new()
    }

    pub fn default(&self) -> Style {
        Style::new()
    }

    pub fn highlighted(&self) -> Style {
        Style::new().fg(Color::LightBlue).bold()
    }

    pub fn focused_border(&self) -> Style {
        border_focus()
    }

    pub fn unfocused_border(&self) -> Style {
        border_unfocus()
    }

    pub fn block(&self, focused: bool, title: impl Into<Title<'static>>) -> Block<'static> {
        let pane_style = if focused {
            self.focused_border()
        } else {
            self.unfocused_border()
        };
        Block::default()
            .borders(Borders::ALL)
            .border_style(pane_style)
            .title(title)
    }

    pub fn scrollbar(&self) -> Scrollbar<'static> {
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None)
    }
}

#[derive(Debug, Clone)]
struct DirectoryStatus {
    dir_name:           CompactString,
    solved_count:       usize,
    unsolved_count:     usize,
    wrong_answer_count: usize,
    not_started_count:  usize,
}

impl DirectoryStatus {
    pub fn total(&self) -> usize {
        self.solved_count + self.unsolved_count + self.wrong_answer_count + self.not_started_count
    }
}

impl From<CompactString> for DirectoryStatus {
    fn from(dir_name: CompactString) -> Self {
        Self {
            dir_name,
            solved_count: 0,
            unsolved_count: 0,
            wrong_answer_count: 0,
            not_started_count: 0,
        }
    }
}

enum TasksNode {
    Task(usize),
    Directory(DirectoryStatus),
}

impl TasksNode {
    pub fn new_task(task_pos: usize) -> Self {
        Self::Task(task_pos)
    }

    pub fn new_directory(dir_name: CompactString) -> Self {
        Self::Directory(dir_name.into())
    }
}

impl Tasks {
    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }

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
        let wrong_answer = self.theme().wrong_answer();
        let unsolved = self.theme().unsolved();
        let solved = self.theme().solved();
        let not_started = self.theme().not_started();
        let default = self.theme().default();

        let text = match tasks_node.data() {
            TasksNode::Task(task) => {
                let task = &self.tasks[*task].task;
                let line_style = match task.solution.status {
                    SolutionStatus::NotDone => not_started,
                    SolutionStatus::Err(_) => unsolved,
                    SolutionStatus::Answer(_) if task.solution.validate_answer() => solved,
                    _ => wrong_answer,
                };

                let conditions = format!("[{}]", task.task.conditions.iter().format(", "),);
                Line::from(vec![
                    Span::styled(task.task.purpose.to_string(), line_style),
                    Span::from(" "),
                    Span::styled(conditions, default),
                ])
            }
            TasksNode::Directory(dir) => Line::from(vec![
                Span::styled(dir.dir_name.to_string(), self.theme().highlighted()),
                Span::from("[".to_owned()),
                Span::styled(format!("{}", dir.not_started_count), not_started),
                Span::styled(format!(" {}", dir.solved_count), solved),
                Span::styled(format!(" {}", dir.unsolved_count), unsolved),
                Span::styled(format!(" {}", dir.wrong_answer_count), wrong_answer),
                Span::from("]".to_owned()),
            ]),
        };

        let children: Vec<_> = tasks_node.iter().map(|s| self.tree(s)).collect();
        if children.is_empty() {
            TreeItem::new_leaf(tasks_node.id(), text)
        } else {
            TreeItem::new(tasks_node.id(), text, children).unwrap()
        }
    }

    fn draw_tasks_list(&mut self, frame: &mut Frame, area: Rect) {
        let items = [self.tree(self.tasks_index.root())];

        let widget = TuiTree::new(&items)
            .expect("all item identifiers are unique")
            .block(self.theme().block(self.focused_pane == 0, "Tasks"))
            .experimental_scrollbar(Some(self.theme().scrollbar()))
            .highlight_style(self.theme().tree_cursor_style())
            .highlight_symbol(">");

        frame.render_stateful_widget(widget, area, &mut self.tasks_tree_state);
    }

    fn draw_solution(&mut self, frame: &mut Frame, area: Rect) {
        let block = self.theme().block(self.focused_pane == 1, "Detailed");

        let Some(selected) = self.tasks_tree_state.selected().last().cloned() else {
            return;
        };
        match self.tasks_index[&selected].data() {
            TasksNode::Task(task_id) => {
                let tracing = self.tasks.get(*task_id).unwrap();
                let mut lines: Vec<_> = format!("Task {}\n\nSolution", tracing.task.task)
                    .split('\n')
                    .map(|x| Line::from(Span::from(x.to_owned())))
                    .collect();

                if tracing.task.solution.status != SolutionStatus::NotDone {
                    lines.extend(
                        // TODO: format
                        { tracing.task.solution.steps() }.map(|x| {
                            Line::from(Span::styled(x.to_string(), self.theme().default()))
                        }),
                    );
                } else {
                    lines.push(Line::from(Span::from("Press s to solve".to_owned())));
                };
                let scroll_pos = tracing.task.scroll_pos.selected().unwrap();
                frame.render_stateful_widget(
                    List::new(lines.iter().cloned())
                        .highlight_style(self.theme().list_cursor_style())
                        .block(block),
                    area,
                    &mut self.tasks[*task_id].task.scroll_pos,
                );

                draw_scrollbar(frame, area, lines.len(), scroll_pos);
            }
            TasksNode::Directory(dir) => {
                frame.render_widget(
                    List::new(self.dir_status_lines(dir))
                        .highlight_style(self.theme().list_cursor_style())
                        .block(block),
                    area,
                );
            }
        };
    }

    fn dir_status_lines(&self, dir: &DirectoryStatus) -> impl Iterator<Item = Line<'static>> {
        let wrong_answer = self.theme().wrong_answer();
        let unsolved = self.theme().unsolved();
        let solved = self.theme().solved();
        let not_started = self.theme().not_started();
        let default = self.theme().default();

        [
            self.pair_line("Group: ", &dir.dir_name, default),
            Line::default(),
            self.pair_line("Total: ", dir.total(), default),
            Line::default(),
            self.pair_line("Not started: ", dir.not_started_count, not_started),
            self.pair_line("Solved: ", dir.solved_count, solved),
            self.pair_line("Not solved: ", dir.unsolved_count, unsolved),
            self.pair_line("Wrong answers: ", dir.wrong_answer_count, wrong_answer),
        ]
        .into_iter()
    }

    fn pair_line<'a>(&self, k: &'a str, v: impl Display, v_style: Style) -> Line<'a> {
        let highlighted = self.theme().highlighted();
        Line::from(vec![
            Span::styled(k, highlighted),
            Span::styled(v.to_string(), v_style),
        ])
    }

    fn add_task(&mut self, rules: Arc<RulesEngine>, task: Task) {
        self.tasks.push(Tracing::new(TaskStatus {
            rules_engine: rules,
            solution: Solution::new(task.clone()).into(),
            task,
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
                    if let TasksNode::Directory(dir) = x.data() {
                        dir.dir_name == i
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
                TasksNode::Directory(dir) => {
                    dir.solved_count = (dir.solved_count as isize + solved_delta) as usize;
                    dir.unsolved_count = (dir.unsolved_count as isize + unsolved_delta) as usize;
                    dir.wrong_answer_count =
                        (dir.wrong_answer_count as isize + wrong_answer_delta) as usize;
                    dir.not_started_count =
                        (dir.not_started_count as isize + not_runned_delta) as usize;
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
        if self.tasks[*task_idx].task.solution.status == SolutionStatus::NotDone {
            not_runned_delta -= 1;
        } else if self.tasks[*task_idx].task.solution.answer().is_none() {
            unsolved_delta -= 1;
        } else if !self.tasks[*task_idx].task.solution.validate_answer() {
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
        task.solution = solver.solve(task.task.clone());

        // Add new task status to remove
        if task.solution.answer().is_none() {
            unsolved_delta += 1;
        } else if !task.solution.as_ref().validate_answer() {
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
}

use std::sync::Arc;

use ratatui::{prelude::*, widgets::ListState};
use trees::{tr, Node, Tree};
use tui_tree_widget::TreeState;

use solver::{
    rule::RulesEngine,
    task::{DumperConfig, SharedSolution, Solution, SolutionStatus, Solver, Task},
};
use utils::{IndexedTree, TreeIndex};

use crate::{
    theme::Theme,
    tracing::Tracing,
    widgets::{
        solution_window::SolutionWindow,
        tasks_list::{TasksList, TasksNode},
    },
};

use super::state::{default_state, Command};

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

    pub fn process(&mut self, command: Command) {
        match command {
            Command::SolveAll => self.solve_node_id(&self.tasks_index.root().id()),
            Command::Solve => {
                if let Some(selected) = self.tasks_tree_state.selected().last().cloned() {
                    self.solve_node_id(&selected);
                }
            }
            Command::Down => {
                if self.focused_pane == 0 {
                    self.tasks_tree_state.key_down();
                } else if let Some(selected) = self.tasks_tree_state.selected().last() {
                    if let TasksNode::Task(tracing) = self.tasks_index[selected].data() {
                        self.tasks[*tracing].task.scroll_pos.select_next()
                    }
                }
            }
            Command::Up => {
                if self.focused_pane == 0 {
                    self.tasks_tree_state.key_up();
                } else if let Some(selected) = self.tasks_tree_state.selected().last() {
                    if let TasksNode::Task(tracing) = self.tasks_index[selected].data() {
                        self.tasks[*tracing].task.scroll_pos.select_previous()
                    }
                }
            }
            Command::Left => self.focused_pane = 0,
            Command::Right => self.focused_pane = 1,
            Command::Toggle if self.focused_pane == 0 => {
                self.tasks_tree_state.toggle_selected();
            }
            _ => {}
        }
    }

    #[inline]
    pub fn replace_rules(&mut self, rules: Arc<RulesEngine>) {
        for task in self.tasks.iter_mut() {
            task.task.rules_engine = rules.clone();
        }
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

        let task_list = TasksList {
            tasks_index: &self.tasks_index,
            tasks:       &self.tasks,
        };
        let block = self.theme().block(self.focused_pane == 0, "Tasks");
        let inner = block.inner(layout[0]);
        frame.render_widget(block, layout[0]);
        frame.render_stateful_widget(task_list, inner, &mut self.tasks_tree_state);

        let block = self.theme().block(self.focused_pane == 1, "Detailed");
        let solution = SolutionWindow {
            tasks_index: &self.tasks_index,
            tasks:       &mut self.tasks,
            selected:    self.tasks_tree_state.selected().last().cloned(),
        };
        let inner = block.inner(layout[1]);
        frame.render_widget(block, layout[1]);
        frame.render_stateful_widget(solution, inner, &mut ());
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

    fn theme(&self) -> &Theme {
        static THEME: Theme = Theme {};
        &THEME
    }
}

use std::{io, sync::Arc};

use ratatui::widgets::ListState;
use trees::{tr, Node, Tree};
use tui_tree_widget::TreeState;

use parser::DirectoryParser;
use solver::{
    rule::RulesEngine,
    task::{DumperConfig, SharedSolution, Solution, SolutionStatus, Solver, Task},
};
use utils::{IndexedTree, TreeIndex};

use super::{settings::Settings, ui::default_state};
use crate::widgets::{tasks_list::TasksNode, tracing_tree::TermId};

pub struct TaskState {
    pub solution:     SharedSolution,
    pub solution_pos: ListState,
    pub tracing_pos:  TreeState<TermId>,
}

pub struct State {
    pub rules_engine: Arc<RulesEngine>,
    pub rules_pos:    ListState,

    pub tasks:            Vec<TaskState>,
    pub tasks_index:      Tree<TasksNode>,
    pub tasks_tree_state: TreeState<TreeIndex>,

    settings: Settings,
}

impl State {
    pub fn try_new(settings: Settings) -> io::Result<Self> {
        let parser = DirectoryParser::new(settings.symbols.clone(), settings.tasks.clone());

        let rules = parser.load_rules().map(Arc::new)?;
        let tasks = parser.load_tasks()?;
        let mut result = Self {
            rules_engine: rules,
            rules_pos: default_state(),
            tasks: Default::default(),
            tasks_index: Tree::new(TasksNode::new_directory("Tasks".into())),
            tasks_tree_state: Default::default(),
            settings,
        };

        for task in tasks.into_iter() {
            result.add_task(task);
        }

        Ok(result)
    }

    pub fn reload(&mut self) -> io::Result<()> {
        self.rules_engine = DirectoryParser::new(&self.settings.symbols, &self.settings.tasks)
            .load_rules()
            .map(Arc::new)?;
        Ok(())
    }

    #[inline]
    pub fn tracing(&mut self) -> Option<&mut TaskState> {
        if let Some(selected) = self.tasks_tree_state.selected().last() {
            if let TasksNode::Task(tracing) = self.tasks_index[selected].data() {
                return self.tasks.get_mut(*tracing);
            }
        }
        None
    }

    fn add_task(&mut self, task: Task) {
        self.tasks.push(TaskState {
            solution:     Solution::new(task.clone()).into(),
            solution_pos: default_state(),
            tracing_pos:  Default::default(),
        });

        let group = self.tasks.last().unwrap().solution.task.group.clone();
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

    pub fn solve_node_id(&mut self, node_id: &TreeIndex) {
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
        if self.tasks[*task_idx].solution.status == SolutionStatus::NotDone {
            not_runned_delta -= 1;
        } else if self.tasks[*task_idx].solution.answer().is_none() {
            unsolved_delta -= 1;
        } else if !self.tasks[*task_idx].solution.validate_answer() {
            wrong_answer_delta -= 1;
        } else {
            solved_delta -= 1;
        };
        let task = &mut self.tasks[*task_idx];
        let mut solver = Solver::new(
            self.rules_engine.clone(),
            DumperConfig {
                sink:     "profiler".into(),
                filename: None,
            }
            .build(),
            self.settings.exec_deadline,
        );
        task.solution = solver.solve(task.solution.task.clone());

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

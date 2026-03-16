use std::{io, sync::Arc};

use ratatui::widgets::ListState;
use trees::{Node, Tree, tr};
use tui_scrollview::ScrollViewState;
use tui_tree_widget::TreeState;

use parser::DirectoryParser;
use solver::{
    CompactString,
    rule::RulesEngine,
    task::{SharedSolution, Solution, SolutionStatus, Task},
};
use utils::{IndexedTree, TreeIndex};

use super::{settings::Settings, ui::default_state};
use crate::widgets::tracing_navigation::TermId;

pub struct State {
    pub rules_engine: Arc<RulesEngine>,
    pub rules_pos:    ListState,

    pub tasks:     Tree<TasksNode>,
    pub tasks_pos: TreeState<TreeIndex>,

    pub solve_queue: Vec<TreeIndex>,

    pub settings: Settings,
}

#[derive(Debug)]
pub enum TasksNode {
    Task(TaskState),
    Directory(DirectoryStat),
}

#[derive(Debug)]
pub struct TaskState {
    pub solution:      SharedSolution,
    pub solution_pos:  ScrollViewState,
    pub tracing_state: Vec<(SharedSolution, TreeState<TermId>)>,
}

#[derive(Debug, Clone)]
pub struct DirectoryStat {
    pub dir_name:           CompactString,
    pub solved_count:       usize,
    pub unsolved_count:     usize,
    pub wrong_answer_count: usize,
    pub not_started_count:  usize,
}

#[derive(Debug, Clone, Default)]
struct DirectoryStatUpdate {
    pub solved_delta:       isize,
    pub unsolved_delta:     isize,
    pub wrong_answer_delta: isize,
    pub not_started_delta:  isize,
}

impl DirectoryStat {
    pub fn total(&self) -> usize {
        self.solved_count + self.unsolved_count + self.wrong_answer_count + self.not_started_count
    }

    fn update(&mut self, upd: &DirectoryStatUpdate) {
        self.solved_count = (self.solved_count as isize + upd.solved_delta) as usize;
        self.unsolved_count = (self.unsolved_count as isize + upd.unsolved_delta) as usize;
        self.wrong_answer_count =
            (self.wrong_answer_count as isize + upd.wrong_answer_delta) as usize;
        self.not_started_count = (self.not_started_count as isize + upd.not_started_delta) as usize;
    }
}

impl From<CompactString> for DirectoryStat {
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

impl TasksNode {
    pub fn new_task(task: TaskState) -> Self {
        Self::Task(task)
    }

    pub fn new_directory(dir_name: CompactString) -> Self {
        Self::Directory(dir_name.into())
    }
}

impl State {
    pub fn try_new(settings: Settings) -> io::Result<Self> {
        let parser = DirectoryParser::new(settings.symbols.clone(), settings.tasks.clone());

        let rules = parser.load_rules().map(Arc::new)?;
        let tasks = parser.load_tasks()?;
        let mut result = Self {
            rules_engine: rules,
            rules_pos: default_state(),
            tasks: Tree::new(TasksNode::new_directory("Tasks".into())),
            tasks_pos: Default::default(),
            settings,
            solve_queue: Default::default(),
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
    pub fn selected_task(&mut self) -> Option<&mut TaskState> {
        let selected = self.tasks_pos.selected().last()?;

        if let TasksNode::Task(tracing) = self.tasks[selected].data_mut() {
            return Some(tracing);
        }
        None
    }

    fn add_task(&mut self, task: Task) {
        let node_id = {
            let node = self.find_node_mut(task.group.as_str());
            node.push_back(tr(TasksNode::new_task(TaskState {
                solution:      Solution::new(task.clone()).into(),
                solution_pos:  Default::default(),
                tracing_state: Default::default(),
            })));
            node.id()
        };
        let upd = DirectoryStatUpdate {
            not_started_delta: 1,
            ..Default::default()
        };
        self.counters_update(&node_id, upd);
    }

    fn find_node_mut<'a>(&'a mut self, path: &str) -> &'a mut Node<TasksNode> {
        let mut current_node = self.tasks.root_mut().get_mut();
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

    fn counters_update(&mut self, node_id: &TreeIndex, upd: DirectoryStatUpdate) {
        let mut node = self.tasks.get_mut(node_id);
        while let Some(n) = node {
            match n.data_mut() {
                TasksNode::Directory(dir) => {
                    dir.update(&upd);
                }
                TasksNode::Task(_) => {}
            }

            // TODO: optimize
            let parent_id = n.parent().map(|x| x.id());
            node = parent_id.and_then(|id| self.tasks.get_mut(&id));
        }
    }

    pub fn mark_to_solve(&mut self, node_id: TreeIndex) {
        let Some(node) = self.tasks.get_mut(&node_id) else {
            return;
        };

        if let TasksNode::Directory { .. } = node.data_mut() {
            self.solve_queue.extend(node.iter().map(|x| x.id()));
        } else {
            self.solve_queue.push(node_id);
        };
    }

    pub fn update_task_solution(&mut self, idx: &TreeIndex, solution: SharedSolution) {
        let Some(node) = self.tasks.get_mut(idx) else {
            return;
        };
        let TasksNode::Task(task) = node.data_mut() else {
            return;
        };

        let mut upd = DirectoryStatUpdate::default();

        // Mark previous task status to remove
        if task.solution.status == SolutionStatus::NotDone {
            upd.not_started_delta -= 1;
        } else if task.solution.answer().is_none() {
            upd.unsolved_delta -= 1;
        } else if !task.solution.validate_answer() {
            upd.wrong_answer_delta -= 1;
        } else {
            upd.solved_delta -= 1;
        };
        task.solution = solution.clone();
        task.tracing_state = vec![(solution, Default::default())];

        // Add new task status to remove
        if task.solution.answer().is_none() {
            upd.unsolved_delta += 1;
        } else if !task.solution.as_ref().validate_answer() {
            upd.wrong_answer_delta += 1;
        } else {
            upd.solved_delta += 1;
        };

        // Propagate changes to all parent nodes
        self.counters_update(idx, upd);
    }
}

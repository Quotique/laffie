use std::{collections::HashSet, io, sync::Arc};

use ratatui::widgets::ListState;
use trees::{Node, Tree, tr};
use tui_scrollview::ScrollViewState;
use tui_tree_widget::TreeState;

use parser::DirectoryParser;
use solver::{
    CompactString,
    engine::{SharedSolution, Solution, SolutionStatus},
    rule::{RulesEngine, SharedRule},
    task::Task,
};
use utils::{IndexedTree, TreeIndex};

use super::{settings::Settings, ui::default_state};
use crate::widgets::tracing_navigation::TermId;

pub struct State {
    pub rules_engine: Arc<RulesEngine>,
    pub rules_pos:    ListState,
    pub rules_filter: String,

    pub tasks:            Tree<TasksNode>,
    pub tasks_pos:        TreeState<TreeIndex>,
    pub dir_solution_pos: ScrollViewState,

    pub solve_queue: Vec<TreeIndex>,

    pub settings:     Settings,
    pub settings_pos: ListState,
}

#[derive(Debug, Clone)]
pub struct ProblemTask {
    pub kind:    TaskStatusKind,
    pub task_id: u64,
    pub text:    String,
}

#[derive(Debug)]
pub enum TasksNode {
    Task(TaskState),
    Directory(DirectoryStat),
}

#[derive(Debug)]
pub struct TaskState {
    pub solution:          SharedSolution,
    pub previous_solution: Option<SharedSolution>,
    pub solution_pos:      ScrollViewState,
    pub tracing_state:     Vec<(SharedSolution, TreeState<TermId>)>,
}

#[derive(Debug, Clone)]
pub struct DirectoryStat {
    pub dir_name:           CompactString,
    pub solved_count:       usize,
    pub unsolved_count:     usize,
    pub wrong_answer_count: usize,
    pub not_started_count:  usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatusKind {
    NotStarted,
    Solved,
    WrongAnswer,
    Unsolved,
}

impl TaskStatusKind {
    pub fn of(solution: &Solution) -> Self {
        match solution.status {
            SolutionStatus::NotDone => Self::NotStarted,
            SolutionStatus::Err(_) => Self::Unsolved,
            SolutionStatus::Answer(_) if solution.validate_answer() => Self::Solved,
            SolutionStatus::Answer(_) => Self::WrongAnswer,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct DirectoryStatUpdate {
    pub solved_delta:       isize,
    pub unsolved_delta:     isize,
    pub wrong_answer_delta: isize,
    pub not_started_delta:  isize,
}

impl DirectoryStatUpdate {
    fn for_transition(from: Option<TaskStatusKind>, to: Option<TaskStatusKind>) -> Self {
        let mut upd = Self::default();
        if let Some(kind) = from {
            *upd.field_mut(kind) -= 1;
        }
        if let Some(kind) = to {
            *upd.field_mut(kind) += 1;
        }
        upd
    }

    fn field_mut(&mut self, kind: TaskStatusKind) -> &mut isize {
        match kind {
            TaskStatusKind::NotStarted => &mut self.not_started_delta,
            TaskStatusKind::Solved => &mut self.solved_delta,
            TaskStatusKind::WrongAnswer => &mut self.wrong_answer_delta,
            TaskStatusKind::Unsolved => &mut self.unsolved_delta,
        }
    }
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

        let rules = Arc::new(parser.load_rules()?.value);
        let tasks = parser.load_tasks()?.value;
        let mut result = Self {
            rules_engine: rules,
            rules_pos: default_state(),
            rules_filter: String::new(),
            tasks: Tree::new(TasksNode::new_directory("Tasks".into())),
            tasks_pos: Default::default(),
            dir_solution_pos: Default::default(),
            settings,
            settings_pos: default_state(),
            solve_queue: Default::default(),
        };

        for task in tasks.into_iter() {
            result.add_task(task);
        }

        Ok(result)
    }

    pub fn filtered_rules(&self) -> Vec<SharedRule> {
        if self.rules_filter.is_empty() {
            return self.rules_engine.iter().collect();
        }
        let needle = self.rules_filter.to_lowercase();
        self.rules_engine
            .iter()
            .filter(|r| {
                r.id.to_string().contains(&needle) ||
                    r.term.to_string().to_lowercase().contains(&needle)
            })
            .collect()
    }

    pub fn reload(&mut self) -> io::Result<()> {
        self.rules_engine = Arc::new(
            DirectoryParser::new(&self.settings.symbols, &self.settings.tasks)
                .load_rules()?
                .value,
        );
        Ok(())
    }

    pub fn reload_tasks(&mut self) -> io::Result<()> {
        let parser = DirectoryParser::new(&self.settings.symbols, &self.settings.tasks);
        let new_tasks = parser.load_tasks()?.value;

        let mut existing: HashSet<u64> = HashSet::new();
        collect_known_task_ids(self.tasks.root(), &mut existing);

        for task in new_tasks {
            if !existing.contains(&task.id) {
                self.add_task(task);
            }
        }
        Ok(())
    }

    pub fn reload_all(&mut self) -> io::Result<()> {
        self.reload()?;
        self.reload_tasks()
    }

    #[inline]
    pub fn selected_task(&mut self) -> Option<&mut TaskState> {
        let selected = self.tasks_pos.selected().last()?;

        if let TasksNode::Task(tracing) = self.tasks[selected].data_mut() {
            return Some(tracing);
        }
        None
    }

    pub fn solution_scroll_mut(&mut self) -> &mut ScrollViewState {
        let idx = self.tasks_pos.selected().last().cloned();
        if let Some(idx) = idx &&
            let TasksNode::Task(task) = self.tasks[&idx].data_mut()
        {
            return &mut task.solution_pos;
        }
        &mut self.dir_solution_pos
    }

    fn add_task(&mut self, task: Task) {
        let solution: SharedSolution = Solution::new(task.clone()).into();
        let kind = TaskStatusKind::of(&solution);
        let node_id = {
            let node = self.find_node_mut(task.group.as_str());
            node.push_back(tr(TasksNode::new_task(TaskState {
                solution,
                previous_solution: None,
                solution_pos: Default::default(),
                tracing_state: Default::default(),
            })));
            node.id()
        };
        let upd = DirectoryStatUpdate::for_transition(None, Some(kind));
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
        self.solve_queue
            .extend(collect_to_solve(&self.tasks, node_id));
    }

    pub fn update_task_solution(&mut self, idx: &TreeIndex, solution: SharedSolution) {
        let Some(node) = self.tasks.get_mut(idx) else {
            return;
        };
        let TasksNode::Task(task) = node.data_mut() else {
            return;
        };

        let from = TaskStatusKind::of(&task.solution);
        if from != TaskStatusKind::NotStarted {
            task.previous_solution = Some(task.solution.clone());
        }
        task.solution = solution.clone();
        task.tracing_state = vec![(solution, Default::default())];
        let to = TaskStatusKind::of(&task.solution);

        let upd = DirectoryStatUpdate::for_transition(Some(from), Some(to));
        self.counters_update(idx, upd);
    }
}

/// Task ids to enqueue when solving `node_id`: the node itself if it is a task,
/// otherwise every leaf task beneath it (recursively, at any depth).
fn collect_to_solve(tasks: &Tree<TasksNode>, node_id: TreeIndex) -> Vec<TreeIndex> {
    let Some(node) = tasks.get(&node_id) else {
        return Vec::new();
    };
    match node.data() {
        TasksNode::Task(_) => vec![node_id],
        TasksNode::Directory(_) => {
            let mut out = Vec::new();
            collect_task_ids(node, &mut out);
            out
        }
    }
}

fn collect_task_ids(node: &Node<TasksNode>, out: &mut Vec<TreeIndex>) {
    for child in node.iter() {
        match child.data() {
            TasksNode::Task(_) => out.push(child.id()),
            TasksNode::Directory(_) => collect_task_ids(child, out),
        }
    }
}

pub fn collect_problem_tasks(node: &Node<TasksNode>, out: &mut Vec<ProblemTask>) {
    for child in node.iter() {
        match child.data() {
            TasksNode::Task(task) => {
                let kind = TaskStatusKind::of(&task.solution);
                if matches!(kind, TaskStatusKind::WrongAnswer | TaskStatusKind::Unsolved) {
                    out.push(ProblemTask {
                        kind,
                        task_id: task.solution.task.id,
                        text: task.solution.task.text.clone(),
                    });
                }
            }
            TasksNode::Directory(_) => collect_problem_tasks(child, out),
        }
    }
}

fn collect_known_task_ids(node: &Node<TasksNode>, out: &mut HashSet<u64>) {
    for child in node.iter() {
        match child.data() {
            TasksNode::Task(task) => {
                out.insert(task.solution.task.id);
            }
            TasksNode::Directory(_) => collect_known_task_ids(child, out),
        }
    }
}

#[cfg(test)]
mod tests {
    use solver::{
        engine::Solution,
        task::{Goal, TaskBuilder},
        term::TermBuf,
    };
    use trees::{Tree, tr};

    use super::*;

    fn dummy_task_state() -> TaskState {
        let goal = TermBuf::symbol("find").arg(TermBuf::variable("x"));
        let task = TaskBuilder::from_goal(Goal::parse(goal).expect("find(x) is a goal")).build();
        TaskState {
            solution:          Solution::new(task).into(),
            previous_solution: None,
            solution_pos:      Default::default(),
            tracing_state:     Vec::new(),
        }
    }

    // root ├─ sub ├─ task ; direct task hangs off root, tasks live two levels deep.
    //      │      └─ task
    //      └─ task
    fn two_level_tree() -> Tree<TasksNode> {
        let mut sub = Tree::new(TasksNode::new_directory("sub".into()));
        sub.push_back(tr(TasksNode::new_task(dummy_task_state())));
        sub.push_back(tr(TasksNode::new_task(dummy_task_state())));

        let mut root = Tree::new(TasksNode::new_directory("root".into()));
        root.push_back(sub);
        root.push_back(tr(TasksNode::new_task(dummy_task_state())));
        root
    }

    #[test]
    fn solve_all_reaches_nested_tasks() {
        let tasks = two_level_tree();
        let ids = collect_to_solve(&tasks, tasks.root().id());
        assert_eq!(ids.len(), 3, "two nested tasks + one direct task");
    }

    #[test]
    fn solving_a_task_node_enqueues_only_itself() {
        let tasks = two_level_tree();
        let direct = tasks.root().iter().nth(1).unwrap().id();
        assert_eq!(collect_to_solve(&tasks, direct.clone()), vec![direct]);
    }

    fn upd_tuple(u: &DirectoryStatUpdate) -> (isize, isize, isize, isize) {
        (
            u.not_started_delta,
            u.solved_delta,
            u.wrong_answer_delta,
            u.unsolved_delta,
        )
    }

    #[test]
    fn add_new_task_increments_not_started() {
        let upd = DirectoryStatUpdate::for_transition(None, Some(TaskStatusKind::NotStarted));
        assert_eq!(upd_tuple(&upd), (1, 0, 0, 0));
    }

    #[test]
    fn not_started_to_solved() {
        let upd = DirectoryStatUpdate::for_transition(
            Some(TaskStatusKind::NotStarted),
            Some(TaskStatusKind::Solved),
        );
        assert_eq!(upd_tuple(&upd), (-1, 1, 0, 0));
    }

    #[test]
    fn not_started_to_wrong_to_solved() {
        let first = DirectoryStatUpdate::for_transition(
            Some(TaskStatusKind::NotStarted),
            Some(TaskStatusKind::WrongAnswer),
        );
        assert_eq!(upd_tuple(&first), (-1, 0, 1, 0));

        let second = DirectoryStatUpdate::for_transition(
            Some(TaskStatusKind::WrongAnswer),
            Some(TaskStatusKind::Solved),
        );
        assert_eq!(upd_tuple(&second), (0, 1, -1, 0));
    }

    #[test]
    fn solved_back_to_unsolved() {
        let upd = DirectoryStatUpdate::for_transition(
            Some(TaskStatusKind::Solved),
            Some(TaskStatusKind::Unsolved),
        );
        assert_eq!(upd_tuple(&upd), (0, -1, 0, 1));
    }

    #[test]
    fn same_kind_is_no_op() {
        for kind in [
            TaskStatusKind::NotStarted,
            TaskStatusKind::Solved,
            TaskStatusKind::WrongAnswer,
            TaskStatusKind::Unsolved,
        ] {
            let upd = DirectoryStatUpdate::for_transition(Some(kind), Some(kind));
            assert_eq!(upd_tuple(&upd), (0, 0, 0, 0), "kind={kind:?}");
        }
    }
}

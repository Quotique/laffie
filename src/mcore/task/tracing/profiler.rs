use std::rc::Rc;

use ego_tree::{NodeId, Tree};

use crate::{
    rule::{SharedRule, Suppose},
    task::{Solution, Task},
    term::Term,
};

use super::Tracer;

#[derive(Clone, Debug, Default)]
pub struct TermProfileInfo {
    pub parent: String, // TODO: Term
    pub rule:   String, // TODO: SharedRule
    pub term:   String, // TODO: Term

    pub params: Vec<(String, String)>,

    pub requirements:   Vec<String>,
    pub first_unproven: usize,

    pub start_cycle: usize,
    pub end_cycle:   usize,
}

#[derive(Clone, Debug, Default)]
pub struct TaskProfileInfo {
    pub purpose: String,

    pub start_cycle: usize,
    pub end_cycle:   usize,
}

#[derive(Clone, Debug)]
pub struct Profiler {
    pub task:     Tree<ProfilerNode>,
    current_node: NodeId,
}

#[derive(Clone, Debug)]
pub enum ProfilerNode {
    Helper(TaskProfileInfo),
    Suppose(TermProfileInfo),
}

impl ProfilerNode {
    #[inline]
    pub fn cycles(&self) -> usize {
        match self {
            Self::Suppose(suppose) => suppose.end_cycle - suppose.start_cycle,
            Self::Helper(task) => task.end_cycle - task.start_cycle,
        }
    }
}

impl Default for Profiler {
    fn default() -> Self {
        let tree = Tree::new(ProfilerNode::Helper(TaskProfileInfo {
            purpose: "Profiler root".to_owned(),

            start_cycle: Default::default(),
            end_cycle:   Default::default(),
        }));
        Self {
            current_node: tree.root().id(),
            task:         tree,
        }
    }
}

impl TaskProfileInfo {
    #[inline]
    pub fn cycles(&self) -> usize {
        self.end_cycle - self.start_cycle
    }
}

impl TermProfileInfo {
    #[inline]
    pub fn cycles(&self) -> usize {
        self.end_cycle - self.start_cycle
    }
}

impl Tracer for Profiler {
    fn on_new_suppose(
        &mut self,
        parent: Rc<Term>,
        rule: SharedRule,
        suppose: &Suppose,
        cycle: usize,
    ) {
        self.current_node = self
            .task
            .get_mut(self.current_node)
            .unwrap()
            .append(ProfilerNode::Suppose(TermProfileInfo {
                parent: parent.to_string(),
                rule:   rule.to_string(),
                term:   suppose.resolution.to_string(),

                params: suppose
                    .params
                    .params()
                    .map(|(param, value)| (param.to_string(), value.to_string()))
                    .collect(),

                requirements:   suppose.requirements.iter().map(|x| x.to_string()).collect(),
                first_unproven: 0,

                start_cycle: cycle,
                end_cycle:   Default::default(),
            }))
            .id();
    }

    fn on_subtask_start(&mut self, task: &Task, cycle: usize) {
        self.current_node = self
            .task
            .get_mut(self.current_node)
            .unwrap()
            .append(ProfilerNode::Helper(TaskProfileInfo {
                purpose: task.purpose.to_string(),

                start_cycle: cycle,
                end_cycle:   Default::default(),
            }))
            .id();
    }

    fn on_subtask_end(&mut self, status: &Solution) {
        match self.task.get_mut(self.current_node).unwrap().value() {
            ProfilerNode::Helper(task) => task.end_cycle = *status.cycles.borrow(),
            ProfilerNode::Suppose(_) => unreachable!("last node is not subtask"),
        }

        if let Some(parent) = self.task.get(self.current_node).unwrap().parent() {
            self.current_node = parent.id();
        }
    }

    fn on_suppose_finish(&mut self, _suppose: &Suppose, cycle: usize, first_unproven: usize) {
        match self.task.get_mut(self.current_node).unwrap().value() {
            ProfilerNode::Helper(_) => unreachable!("last node is not suppose"),
            ProfilerNode::Suppose(suppose) => {
                suppose.end_cycle = cycle;
                suppose.first_unproven = first_unproven;
            }
        }

        if let Some(parent) = self.task.get(self.current_node).unwrap().parent() {
            self.current_node = parent.id();
        }
    }
}

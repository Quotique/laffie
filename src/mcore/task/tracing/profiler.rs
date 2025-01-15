use ego_tree::{NodeId, Tree};

use crate::{
    rule::{SharedRule, Suppose},
    task::{Solution, Task},
};

use super::Tracer;

#[derive(Clone, Debug, Default)]
pub struct TermProfileInfo {
    pub rule: String, // TODO: SharedRule
    pub term: String, // TODO: Term
}

#[derive(Clone, Debug, Default)]
pub struct TaskProfileInfo {
    pub purpose: String,
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

impl Default for Profiler {
    fn default() -> Self {
        let tree = Tree::new(ProfilerNode::Helper(TaskProfileInfo {
            purpose: "Profiler root".to_owned(),
        }));
        Self {
            current_node: tree.root().id(),
            task:         tree,
        }
    }
}

impl Tracer for Profiler {
    fn on_new_suppose(&mut self, rule: SharedRule, suppose: &Suppose) {
        self.current_node = self
            .task
            .get_mut(self.current_node)
            .unwrap()
            .append(ProfilerNode::Suppose(TermProfileInfo {
                rule: rule.to_string(),
                term: format!("suppose {}", suppose.resolution),
            }))
            .id();
    }

    fn on_subtask_start(&mut self, task: &Task, _cycle: usize) {
        self.current_node = self
            .task
            .get_mut(self.current_node)
            .unwrap()
            .append(ProfilerNode::Helper(TaskProfileInfo {
                purpose: format!("task {}", task.purpose),
            }))
            .id();
    }

    fn on_subtask_end(&mut self, _status: &Solution) {
        if let Some(parent) = self.task.get(self.current_node).unwrap().parent() {
            self.current_node = parent.id();
        }
    }

    fn on_suppose_finish(&mut self, _suppose: &Suppose, _result: bool) {
        if let Some(parent) = self.task.get(self.current_node).unwrap().parent() {
            self.current_node = parent.id();
        }
    }
}

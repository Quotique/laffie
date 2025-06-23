use std::rc::Rc;

use ego_tree::{NodeId, Tree};

use crate::{
    rule::{Hypothesis, SharedRule},
    task::{Solver, Task},
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
    pub purpose: String, // TODO: Term
    pub answer:  Option<String>,

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
    Hypothesis(TermProfileInfo),
}

impl ProfilerNode {
    #[inline]
    pub fn cycles(&self) -> usize {
        match self {
            Self::Hypothesis(hypothesis) => hypothesis.end_cycle - hypothesis.start_cycle,
            Self::Helper(task) => task.end_cycle - task.start_cycle,
        }
    }
}

impl Default for Profiler {
    fn default() -> Self {
        let tree = Tree::new(ProfilerNode::Helper(TaskProfileInfo {
            purpose: "Solution".to_owned(),
            answer:  Some("".to_owned()),

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
    fn on_new_hypothesis(
        &mut self,
        parent: Rc<Term>,
        rule: SharedRule,
        hypothesis: &Hypothesis,
        cycle: usize,
    ) {
        let mut term = hypothesis.resolution.to_string();
        if term == "true" || term == "false" {
            term = format!("{parent} <=> {term}");
        }

        self.current_node = self
            .task
            .get_mut(self.current_node)
            .unwrap()
            .append(ProfilerNode::Hypothesis(TermProfileInfo {
                parent: parent.to_string(),
                rule: rule.to_string(),
                term,

                params: hypothesis
                    .params
                    .params
                    .iter()
                    .map(|(param, value)| (param.to_string(), value.as_subterm().to_string()))
                    .collect(),

                requirements: hypothesis
                    .requirements
                    .iter()
                    .map(|x| x.to_string())
                    .collect(),
                first_unproven: 0,

                start_cycle: cycle,
                end_cycle: Default::default(),
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
                answer:  None,

                start_cycle: cycle,
                end_cycle:   Default::default(),
            }))
            .id();
    }

    fn on_subtask_end(&mut self, status: &Solver) {
        match self.task.get_mut(self.current_node).unwrap().value() {
            ProfilerNode::Helper(task) => {
                task.end_cycle = *status.cycles.borrow();
                task.answer = status.answer().map(|x| x.to_string());
            }
            ProfilerNode::Hypothesis(_) => unreachable!("last node is not subtask"),
        }

        if let Some(parent) = self.task.get(self.current_node).unwrap().parent() {
            self.current_node = parent.id();
        }
    }

    fn on_hypothesis_finish(
        &mut self,
        _hypothesis: &Hypothesis,
        cycle: usize,
        first_unproven: usize,
    ) {
        match self.task.get_mut(self.current_node).unwrap().value() {
            ProfilerNode::Helper(_) => unreachable!("last node is not hypothesis"),
            ProfilerNode::Hypothesis(hypothesis) => {
                hypothesis.end_cycle = cycle;
                hypothesis.first_unproven = first_unproven;
            }
        }

        if let Some(parent) = self.task.get(self.current_node).unwrap().parent() {
            self.current_node = parent.id();
        }
    }
}

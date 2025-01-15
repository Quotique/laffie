use crate::{
    rule::{SharedRule, Suppose},
    task::{Solution, Task},
};

use super::Tracer;

#[derive(Clone, Debug, Default)]
pub struct TermProfileInfo {
    pub rule:     String, // TODO: SharedRule
    pub term:     String, // TODO: Term
    pub supposes: Vec<TaskProfileInfo>,
}

#[derive(Clone, Debug)]
pub struct TaskProfileInfo {
    pub purpose: String,
    pub terms:   Vec<TermProfileInfo>,
}

#[derive(Clone, Debug)]
pub struct Profiler {
    pub task:          TaskProfileInfo,
    current_task_path: Vec<usize>,
}

#[derive(Clone, Debug)]
pub enum ProfilerNode {
    Helper(TaskProfileInfo),
    Suppose(TermProfileInfo),
}

impl Default for Profiler {
    fn default() -> Self {
        Self {
            task:              TaskProfileInfo {
                purpose: Default::default(),
                terms:   Default::default(),
            },
            current_task_path: Default::default(),
        }
    }
}

impl Tracer for Profiler {
    fn on_new_suppose(&mut self, rule: SharedRule, suppose: &Suppose) {
        error!("new suppose {}", suppose);
        error!("current\n{}", Self::tree_view(&self.task, 0));

        self.current_task().terms.push(TermProfileInfo {
            rule:     rule.to_string(),
            term:     format!("suppose {}", suppose.resolution),
            supposes: Default::default(),
        });
        error!("current after\n{}", Self::tree_view(&self.task, 0));
    }

    fn on_subtask_start(&mut self, task: &Task, _cycle: usize) {
        error!("new subtask {}", task);
        error!("current\n{}", Self::tree_view(&self.task, 0));

        let pos = {
            let current_task = self.current_task();
            if current_task.terms.is_empty() {
                current_task.terms.push(TermProfileInfo {
                    rule:     task
                        .purpose
                        .rule
                        .as_ref()
                        .map(|x| x.to_string())
                        .unwrap_or_default(),
                    term:     format!("sym {}", task.purpose.term.to_string()),
                    supposes: Default::default(),
                });
            }
            let current_suppose = current_task.terms.last_mut().unwrap();
            current_suppose.supposes.push(TaskProfileInfo {
                purpose: format!("task {}", task.purpose),
                terms:   vec![],
            });
            current_suppose.supposes.len() - 1
        };
        self.current_task_path.push(pos);
    }

    fn on_subtask_end(&mut self, _status: &Solution) {
        error!("subtask ends ");
        error!("current\n{}", Self::tree_view(&self.task, 0));

        let _ = self.current_task_path.pop();
    }
}

impl Profiler {
    fn current_task(&mut self) -> &mut TaskProfileInfo {
        error!("current_task {:?}", self.current_task_path);
        error!("current\n{}", Self::tree_view(&self.task, 0));

        let mut current = &mut self.task;

        for i in self.current_task_path.iter() {
            current = current
                .terms
                .last_mut()
                .unwrap()
                .supposes
                .get_mut(*i)
                .unwrap();
        }

        current
    }

    fn tree_view(task: &TaskProfileInfo, level: usize) -> String {
        let mut result = String::default();
        for term in task.terms.iter() {
            result = format!("{}\n{}{}", result, " ".repeat(level), term.term);
            for sup in term.supposes.iter() {
                result = format!("{}{}", result, Self::tree_view(sup, level + 1));
            }
        }
        result
    }
}

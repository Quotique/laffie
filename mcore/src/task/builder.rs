use crate::term::TermProps;
use std::{collections::HashMap, fmt, iter::Iterator};

use super::Task;

#[derive(Clone, Debug)]
pub enum TaskBuilderError {
    OnlyOnePurposeAllowed,
    NoPurposeFound,
}

#[derive(Default)]
pub struct TaskBuilder {
    id:          u64,
    conditions:  Vec<TermProps>,
    purpose:     Option<TermProps>,
    term_id_map: HashMap<usize, usize>,

    subtask_level: usize,
}

impl TaskBuilder {
    pub fn with_id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }

    pub fn with_purpose(mut self, purpose: TermProps) -> Result<Self, TaskBuilderError> {
        if let Some(_x) = self.purpose.replace(purpose) {
            Err(TaskBuilderError::OnlyOnePurposeAllowed)
        } else {
            Ok(self)
        }
    }

    pub fn with_condition(mut self, mut condition: TermProps) -> Self {
        self.term_id_map.insert(condition.id, self.conditions.len());
        condition.weight = 0;
        condition.id = self.conditions.len();

        if let Some(parent) = condition.parent {
            condition.parent = self.term_id_map.get(&parent).cloned();
        }
        self.conditions.push(condition);
        self
    }

    pub fn with_conditions(mut self, reqs: impl Iterator<Item = TermProps>) -> Self {
        for i in reqs {
            self = self.with_condition(i);
        }
        self
    }

    pub fn with_level(mut self, level: usize) -> Self {
        self.subtask_level = level;
        self
    }

    pub fn build(self) -> Result<Task, TaskBuilderError> {
        Ok(Task {
            id:            self.id,
            conditions:    self.conditions,
            purpose:       self.purpose.ok_or(TaskBuilderError::NoPurposeFound)?,
            subtask_level: self.subtask_level,
        })
    }
}

impl fmt::Display for TaskBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OnlyOnePurposeAllowed => write!(f, "Duplicate purpose"),
            Self::NoPurposeFound => write!(f, "No purpose found"),
        }
    }
}

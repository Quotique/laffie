use std::{collections::HashMap, fmt, iter::Iterator};

use super::{Task, TermInference, TermProps};
use crate::term::TermBuf;

#[derive(Clone, Debug)]
pub enum TaskBuilderError {
    OnlyOneGoalAllowed,
    NoGoalFound,
}

#[derive(Default)]
pub struct TaskBuilder {
    id:          u64,
    name:        String,
    text:        String,
    conditions:  Vec<TermProps>,
    goal:        Option<TermProps>,
    term_id_map: HashMap<usize, usize>,

    possible_answers: Vec<TermBuf>,

    subtask_level: usize,
}

impl TaskBuilder {
    pub fn with_id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.text = text;
        self
    }

    pub fn with_goal(mut self, goal: TermProps) -> Result<Self, TaskBuilderError> {
        if let Some(_x) = self.goal.replace(goal) {
            Err(TaskBuilderError::OnlyOneGoalAllowed)
        } else {
            Ok(self)
        }
    }

    pub fn with_condition(mut self, mut condition: TermProps) -> Self {
        self.term_id_map.insert(condition.id, self.conditions.len());
        condition.filters.level = 0.into();
        condition.id = self.conditions.len();

        let parent = match &condition.inference {
            TermInference::Rule { parent, .. } | TermInference::Transform { parent, .. } => {
                Some(*parent)
            }
            TermInference::Condition => None,
        };
        if let Some(parent) = parent {
            match self.term_id_map.get(&parent) {
                Some(&new_parent) => match &mut condition.inference {
                    TermInference::Rule { parent, .. } |
                    TermInference::Transform { parent, .. } => *parent = new_parent,
                    TermInference::Condition => {}
                },
                // Parent wasn't copied into the subtask: keep the term without one.
                None => condition.inference = TermInference::Condition,
            }
        }
        self.conditions.push(condition);
        self
    }

    #[inline]
    pub fn with_conditions(mut self, reqs: impl Iterator<Item = TermProps>) -> Self {
        for i in reqs {
            self = self.with_condition(i);
        }
        self
    }

    #[inline]
    pub fn with_answer(mut self, answer: TermBuf) -> Self {
        self.possible_answers.push(answer);
        self
    }

    #[inline]
    pub fn with_level(mut self, level: usize) -> Self {
        self.subtask_level = level;
        self
    }

    #[inline]
    pub fn build(self) -> Result<Task, TaskBuilderError> {
        Ok(Task {
            id:               self.id,
            name:             self.name,
            text:             self.text,
            group:            "".to_owned(),
            givens:           self.conditions,
            goal:             self.goal.ok_or(TaskBuilderError::NoGoalFound)?,
            subtask_level:    self.subtask_level,
            possible_answers: self.possible_answers,
        })
    }
}

impl fmt::Display for TaskBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OnlyOneGoalAllowed => write!(f, "Duplicate goal"),
            Self::NoGoalFound => write!(f, "No goal found"),
        }
    }
}

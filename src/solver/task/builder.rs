use std::iter::Iterator;

use super::{Goal, Task};
use crate::term::{SharedTerm, TermBuf};

pub struct TaskBuilder {
    id:         u64,
    name:       String,
    text:       String,
    conditions: Vec<SharedTerm>,
    goal:       Goal,

    possible_answers: Vec<TermBuf>,

    subtask_level: usize,
}

impl TaskBuilder {
    pub fn from_goal(goal: Goal) -> Self {
        Self {
            id: Default::default(),
            name: Default::default(),
            text: Default::default(),
            conditions: Default::default(),
            goal,
            possible_answers: Default::default(),
            subtask_level: Default::default(),
        }
    }

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

    pub fn with_condition(mut self, condition: SharedTerm) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn with_conditions(mut self, reqs: impl Iterator<Item = SharedTerm>) -> Self {
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
    pub fn build(self) -> Task {
        let mut task = Task::from_goal(self.goal);
        task.id = self.id;
        task.name = self.name;
        task.text = self.text;
        task.givens = self.conditions;
        task.subtask_level = self.subtask_level;
        task.possible_answers = self.possible_answers;
        task
    }
}

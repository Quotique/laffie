use std::{
    collections::{HashMap, HashSet},
    iter::Iterator,
};

use super::{Goal, Task, TermInference, TermProps};
use crate::{rule::RuleId, term::TermBuf};

pub struct TaskBuilder {
    id:          u64,
    name:        String,
    text:        String,
    conditions:  Vec<TermProps>,
    goal:        Goal,
    term_id_map: HashMap<usize, usize>,

    blocked_rules:    HashSet<RuleId>,
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
            term_id_map: Default::default(),
            blocked_rules: Default::default(),
            possible_answers: Default::default(),
            subtask_level: Default::default(),
        }
    }

    /// A rule that blocked itself on the term a subtask was spawned from stays
    /// blocked inside that subtask, or it fires again there and the search
    /// loops.
    pub(crate) fn with_blocked_rules(mut self, rules: HashSet<RuleId>) -> Self {
        self.blocked_rules = rules;
        self
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
    pub fn build(self) -> Task {
        let mut task = Task::from_goal(self.goal);
        task.id = self.id;
        task.name = self.name;
        task.text = self.text;
        task.givens = self.conditions;
        task.subtask_level = self.subtask_level;
        task.possible_answers = self.possible_answers;
        task.block_rules(self.blocked_rules);
        task
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::TaskBuilder;
    use crate::{
        rule::RuleId,
        task::{Goal, Solution},
        term::{TermBuf, term_with_vars},
    };

    #[test]
    fn blocked_rules_reach_the_goal_term_the_search_starts_from() {
        // Or the rule that blocked itself fires again inside the subtask.
        let blocked = RuleId::new(0, 7);
        let goal = Goal::parse(TermBuf::symbol("transform").arg(term_with_vars("1 + 2")))
            .expect("transform(1 + 2) is a goal");
        let task = TaskBuilder::from_goal(goal)
            .with_blocked_rules(HashSet::from([blocked]))
            .build();

        let solution = Solution::new(task);
        let goal_term = solution
            .terms
            .iter()
            .find(|t| t.filters.is_goal())
            .expect("the goal term is in the solution");
        assert!(goal_term.filters.blocked_rules.contains(&blocked));
    }
}

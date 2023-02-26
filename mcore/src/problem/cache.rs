use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

use crate::statement::Statement;

use super::solution::Solution;

#[derive(Clone)]
pub enum ProblemStatus {
    Solved(Arc<Solution>),
    NotSolved,
    InProgress,
}

#[derive(Default)]
pub struct ProblemsCache {
    problems: RwLock<HashMap<Statement, ProblemStatus>>,
}

impl ProblemStatus {
    pub fn solution(&self) -> Option<Arc<Solution>> {
        match self {
            ProblemStatus::Solved(solution) => Some(solution.clone()),
            _ => None,
        }
    }
}

impl ProblemsCache {
    pub fn contains(&self, target: &Statement) -> bool {
        self.problems.read().contains_key(target)
    }

    pub fn add(&self, target: Statement) -> bool {
        if self.contains(&target) {
            return false;
        }
        self.problems
            .write()
            .insert(target, ProblemStatus::InProgress);
        true
    }

    pub fn status(&self, target: &Statement) -> Option<ProblemStatus> {
        self.problems.read().get(target).cloned()
    }

    pub fn update_status(&self, target: &Statement, status: ProblemStatus) {
        if let Some(s) = self.problems.write().get_mut(target) {
            *s = status;
        } else {
            warn!("attempt to update status for unknown target {}", target);
        }
    }
}

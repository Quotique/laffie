use std::{collections::HashMap, fmt, rc::Rc};

use parking_lot::RwLock;

use crate::{task::Solution, term::Term};

#[derive(Clone)]
pub enum TaskStatus {
    Solved(Rc<Solution>),
    NotSolved,
    InProgress,
}

#[derive(Default)]
pub struct TasksCache {
    tasks: RwLock<HashMap<Term, TaskStatus>>,
}

impl fmt::Debug for TasksCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TasksCache")
    }
}

impl TaskStatus {
    pub fn solver(&self) -> Option<Rc<Solution>> {
        match self {
            TaskStatus::Solved(solution) => Some(solution.clone()),
            _ => None,
        }
    }
}

impl TasksCache {
    pub fn contains(&self, purpose: &Term) -> bool {
        self.tasks.read().contains_key(purpose)
    }

    pub fn add(&self, purpose: Term) -> bool {
        if self.contains(&purpose) {
            return false;
        }
        self.tasks.write().insert(purpose, TaskStatus::InProgress);
        true
    }

    pub fn remove(&self, purpose: &Term) -> Option<TaskStatus> {
        self.tasks.write().remove(purpose)
    }

    pub fn status(&self, purpose: &Term) -> Option<TaskStatus> {
        self.tasks.read().get(purpose).cloned()
    }

    pub fn update_status(&self, purpose: &Term, status: TaskStatus) {
        if let Some(s) = self.tasks.write().get_mut(purpose) {
            *s = status;
        } else {
            warn!("attempt to update status for unknown purpose {}", purpose);
        }
    }

    pub fn clear(&self) {
        self.tasks.write().clear();
    }
}

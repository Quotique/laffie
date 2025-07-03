use std::{collections::HashMap, fmt, rc::Rc};

use parking_lot::RwLock;

use crate::{task::Solution, term::Term};

#[derive(Default)]
pub struct TasksCache {
    tasks: RwLock<HashMap<Term, Option<Rc<Solution>>>>,
}

impl fmt::Debug for TasksCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TasksCache")
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
        self.tasks.write().insert(purpose, None);
        true
    }

    pub fn remove(&self, purpose: &Term) {
        self.tasks.write().remove(purpose);
    }

    pub fn status(&self, purpose: &Term) -> Option<Option<Rc<Solution>>> {
        self.tasks.read().get(purpose).cloned()
    }

    pub fn update_status(&self, purpose: &Term, solution: Rc<Solution>) {
        if let Some(s) = self.tasks.write().get_mut(purpose) {
            *s = Some(solution);
        } else {
            warn!("attempt to update status for unknown purpose {}", purpose);
        }
    }

    pub fn clear(&self) {
        self.tasks.write().clear();
    }
}

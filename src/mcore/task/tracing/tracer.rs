use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    rule::{Rule, Suppose},
    task::{Solution, Task},
    term::TermProps,
};

pub trait Tracer: Send + Sync {
    // Called each time when new task spawned
    fn on_subtask_start(&mut self, _task: &Task, _cycle: usize) {}

    // Called each time when task finished
    fn on_subtask_end(&mut self, _status: &Solution) {}

    // Called each time when new term was added to the solution frame
    fn on_new_term(&mut self, _term: &TermProps, _parent: &TermProps) {}

    // Called on each solution cycle iteration with picked term argument
    fn on_term_focus(&mut self, _term: &TermProps) {}

    // Called on each attempt to apply rule
    fn on_rule_selection(&mut self, _rule: &Rule) {}

    // Called on each new suppose
    fn on_new_suppose(&mut self, _rule: &Rule, _suppose: &Suppose) {}
}

#[derive(Clone)]
pub struct SolutionTracer {
    sink: Arc<Mutex<Box<dyn Tracer>>>,
}
pub struct EmptyTracer {}

impl Tracer for EmptyTracer {}

impl SolutionTracer {
    pub fn new(tracer: impl Tracer + 'static) -> Self {
        Self {
            sink: Arc::new(Mutex::new(Box::new(tracer))),
        }
    }
}

impl Default for SolutionTracer {
    fn default() -> Self {
        Self {
            sink: Arc::new(Mutex::new(Box::new(EmptyTracer {}))),
        }
    }
}

impl Tracer for SolutionTracer {
    fn on_subtask_start(&mut self, task: &Task, cycle: usize) {
        self.sink.lock().on_subtask_start(task, cycle);
    }

    fn on_subtask_end(&mut self, status: &Solution) {
        self.sink.lock().on_subtask_end(status);
    }

    fn on_new_term(&mut self, term: &TermProps, parent: &TermProps) {
        self.sink.lock().on_new_term(term, parent);
    }

    fn on_term_focus(&mut self, term: &TermProps) {
        self.sink.lock().on_term_focus(term);
    }

    fn on_rule_selection(&mut self, rule: &Rule) {
        self.sink.lock().on_rule_selection(rule);
    }

    fn on_new_suppose(&mut self, rule: &Rule, suppose: &Suppose) {
        self.sink.lock().on_new_suppose(rule, suppose);
    }
}

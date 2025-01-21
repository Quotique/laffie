use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    rule::{SharedRule, Suppose},
    task::{Solution, Task},
    term::TermProps,
};

use super::profiler::Profiler;

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
    fn on_rule_selection(&mut self, _rule: SharedRule) {}

    // Called on each new suppose
    fn on_new_suppose(&mut self, _rule: SharedRule, _suppose: &Suppose, _cycle: usize) {}

    // Called on suppose processing finished
    fn on_suppose_finish(&mut self, _suppose: &Suppose, _cycle: usize, _result: bool) {}
}

#[derive(Clone)]
pub struct SolutionTracer {
    sink:     Arc<Mutex<Box<dyn Tracer>>>,
    profiler: Option<Arc<Mutex<Profiler>>>,
}

pub struct EmptyTracer {}

impl Tracer for EmptyTracer {}

impl SolutionTracer {
    pub fn new(tracer: impl Tracer + 'static, use_profiler: bool) -> Self {
        Self {
            sink:     Arc::new(Mutex::new(Box::new(tracer))),
            profiler: if use_profiler {
                Some(Arc::new(Mutex::new(Profiler::default())))
            } else {
                None
            },
        }
    }

    #[inline]
    pub fn profiler(&self) -> Option<&Mutex<Profiler>> {
        self.profiler.as_ref().map(|x| x.as_ref())
    }
}

impl Default for SolutionTracer {
    fn default() -> Self {
        Self {
            sink:     Arc::new(Mutex::new(Box::new(EmptyTracer {}))),
            profiler: None,
        }
    }
}

impl Tracer for SolutionTracer {
    fn on_subtask_start(&mut self, task: &Task, cycle: usize) {
        self.sink.lock().on_subtask_start(task, cycle);
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_subtask_start(task, cycle);
        }
    }

    fn on_subtask_end(&mut self, status: &Solution) {
        self.sink.lock().on_subtask_end(status);
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_subtask_end(status);
        }
    }

    fn on_new_term(&mut self, term: &TermProps, parent: &TermProps) {
        self.sink.lock().on_new_term(term, parent);
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_new_term(term, parent);
        }
    }

    fn on_term_focus(&mut self, term: &TermProps) {
        self.sink.lock().on_term_focus(term);
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_term_focus(term);
        }
    }

    fn on_rule_selection(&mut self, rule: SharedRule) {
        self.sink.lock().on_rule_selection(rule.clone());
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_rule_selection(rule);
        }
    }

    fn on_new_suppose(&mut self, rule: SharedRule, suppose: &Suppose, cycle: usize) {
        self.sink
            .lock()
            .on_new_suppose(rule.clone(), suppose, cycle);
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_new_suppose(rule, suppose, cycle);
        }
    }

    fn on_suppose_finish(&mut self, suppose: &Suppose, cycle: usize, result: bool) {
        self.sink.lock().on_suppose_finish(suppose, cycle, result);
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_suppose_finish(suppose, cycle, result);
        }
    }
}

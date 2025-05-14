use std::{rc::Rc, sync::Arc};

use parking_lot::Mutex;

use crate::{
    rule::{Hypothesis, SharedRule},
    task::{Solver, Task},
    term::{Term, TermProps},
};

use super::profiler::Profiler;

pub trait Tracer: Send + Sync {
    // Called each time when new task spawned
    fn on_subtask_start(&mut self, _task: &Task, _cycle: usize) {}

    // Called each time when task finished
    fn on_subtask_end(&mut self, _status: &Solver) {}

    // Called each time when new term was added to the solution frame
    fn on_new_term(&mut self, _term: &TermProps, _parent: &TermProps) {}

    // Called on each solution cycle iteration with picked term argument
    fn on_term_focus(&mut self, _term: &TermProps) {}

    // Called on each attempt to apply rule
    fn on_rule_selection(&mut self, _rule: SharedRule) {}

    // Called on each new hypothesis
    fn on_new_hypothesis(
        &mut self,
        _parent: Rc<Term>,
        _rule: SharedRule,
        _hypothesis: &Hypothesis,
        _cycle: usize,
    ) {
    }

    // Called on hypothesis processing finished
    fn on_hypothesis_finish(
        &mut self,
        _hypothesis: &Hypothesis,
        _cycle: usize,
        _first_unproven: usize,
    ) {
    }
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

impl SolutionTracer {
    pub fn on_subtask_start(&self, task: &Task, cycle: usize) {
        self.sink.lock().on_subtask_start(task, cycle);
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_subtask_start(task, cycle);
        }
    }

    pub fn on_subtask_end(&self, status: &Solver) {
        self.sink.lock().on_subtask_end(status);
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_subtask_end(status);
        }
    }

    pub fn on_new_term(&self, term: &TermProps, parent: &TermProps) {
        self.sink.lock().on_new_term(term, parent);
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_new_term(term, parent);
        }
    }

    pub fn on_term_focus(&self, term: &TermProps) {
        self.sink.lock().on_term_focus(term);
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_term_focus(term);
        }
    }

    pub fn on_rule_selection(&self, rule: SharedRule) {
        self.sink.lock().on_rule_selection(rule.clone());
        if let Some(profiler) = self.profiler() {
            profiler.lock().on_rule_selection(rule);
        }
    }

    pub fn on_new_hypothesis(
        &self,
        parent: Rc<Term>,
        rule: SharedRule,
        hypothesis: &Hypothesis,
        cycle: usize,
    ) {
        self.sink
            .lock()
            .on_new_hypothesis(parent.clone(), rule.clone(), hypothesis, cycle);
        if let Some(profiler) = self.profiler() {
            profiler
                .lock()
                .on_new_hypothesis(parent, rule, hypothesis, cycle);
        }
    }

    pub fn on_hypothesis_finish(
        &self,
        hypothesis: &Hypothesis,
        cycle: usize,
        first_unproven: usize,
    ) {
        self.sink
            .lock()
            .on_hypothesis_finish(hypothesis, cycle, first_unproven);
        if let Some(profiler) = self.profiler() {
            profiler
                .lock()
                .on_hypothesis_finish(hypothesis, cycle, first_unproven);
        }
    }
}

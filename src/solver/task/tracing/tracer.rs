use std::{rc::Rc, sync::Arc};

use parking_lot::Mutex;

use crate::{
    rule::{Hypothesis, SharedRule},
    task::{Solution, Task, TermProps},
    term::Term,
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

impl SolutionTracer {
    pub fn on_subtask_start(&self, task: &Task, cycle: usize) {
        self.sink.lock().on_subtask_start(task, cycle);
    }

    pub fn on_subtask_end(&self, status: &Solution) {
        self.sink.lock().on_subtask_end(status);
    }

    pub fn on_new_term(&self, term: &TermProps, parent: &TermProps) {
        self.sink.lock().on_new_term(term, parent);
    }

    pub fn on_term_focus(&self, term: &TermProps) {
        self.sink.lock().on_term_focus(term);
    }

    pub fn on_rule_selection(&self, rule: SharedRule) {
        self.sink.lock().on_rule_selection(rule.clone());
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
    }
}

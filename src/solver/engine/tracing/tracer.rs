use crate::{
    engine::{Solution, TermInference, TermProps},
    rule::GroundedHypothesis,
    task::Task,
    term::SharedTerm,
};

use super::file::FileDumpTracer;

pub trait Tracer: Send + Sync {
    // Called each time when new task spawned
    fn on_subtask_start(&mut self, _task: &Task, _cycle: usize) {}

    // Called each time when task finished
    fn on_subtask_end(&mut self, _status: &Solution) {}

    // Called each time when new term was added to the solution frame
    fn on_new_term(&mut self, _term: &TermProps, _parent: &TermProps) {}

    // Called on each solution cycle iteration with picked term argument
    fn on_term_focus(&mut self, _term: &TermProps, _cycle: usize) {}

    // Called on each new hypothesis
    fn on_new_hypothesis(
        &mut self,
        _parent: SharedTerm,
        _hypothesis: &GroundedHypothesis,
        _cycle: usize,
    ) {
    }

    // Called on hypothesis processing finished
    fn on_hypothesis_finish(&mut self, _inference: &TermInference, _cycle: usize) {}
}

#[derive(Default)]
pub struct TracerHub {
    tracers: Vec<Box<dyn Tracer>>,
}

impl TracerHub {
    pub fn new() -> Self {
        Self {
            tracers: Default::default(),
        }
    }

    pub fn add_custom(&mut self, tracer: impl Tracer + 'static) -> &mut Self {
        self.tracers.push(Box::new(tracer));
        self
    }

    pub fn add_file_dumper(&mut self, filename: impl AsRef<str>) -> &mut Self {
        self.tracers
            .push(Box::new(FileDumpTracer::new(filename.as_ref())));
        self
    }
}

impl Tracer for TracerHub {
    fn on_subtask_start(&mut self, task: &Task, cycle: usize) {
        for i in self.tracers.iter_mut() {
            i.on_subtask_start(task, cycle);
        }
    }

    fn on_subtask_end(&mut self, status: &Solution) {
        for i in self.tracers.iter_mut() {
            i.on_subtask_end(status);
        }
    }

    fn on_new_term(&mut self, term: &TermProps, parent: &TermProps) {
        for i in self.tracers.iter_mut() {
            i.on_new_term(term, parent);
        }
    }

    fn on_term_focus(&mut self, term: &TermProps, cycle: usize) {
        for i in self.tracers.iter_mut() {
            i.on_term_focus(term, cycle);
        }
    }

    fn on_new_hypothesis(
        &mut self,
        parent: SharedTerm,
        hypothesis: &GroundedHypothesis,
        cycle: usize,
    ) {
        for i in self.tracers.iter_mut() {
            i.on_new_hypothesis(parent.clone(), hypothesis, cycle);
        }
    }

    fn on_hypothesis_finish(&mut self, inference: &TermInference, cycle: usize) {
        for i in self.tracers.iter_mut() {
            i.on_hypothesis_finish(inference, cycle);
        }
    }
}

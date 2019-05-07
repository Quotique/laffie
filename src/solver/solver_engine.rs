use Problem;
use Solution;

use core::rules_engine::RulesEngine;

pub struct SolverEngine<'a> {
    pub problem: &'a Problem,
    pub solution: Vec<Statement>,
    pub result: Solution,
}

impl<'a> SolverEngine<'a> {
    pub fn new(problem: &'a Problem) -> SolverEngine<'a> {
        SolverEngine(problem: problem, solution: vec![], result: Solution())
    }

    pub fn run(rules: &RulesEngine) -> Option<Solution> {
    }
}

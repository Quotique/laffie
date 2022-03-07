use std::{
    fmt,
    sync::Arc,
    time::{Instant, SystemTime},
};

use bincode::{Decode, Encode};

use crate::{
    rule::{RulesEngine, SharedRule},
    statement::Statement,
    utils::{Dumper, DumperSink},
};

use super::{frame::Frame, problem::Problem, target::Target};

pub const MAX_SUBPROBLEM_LEVEL: usize = 10;
pub const MAX_LEVEL: usize = 20;

#[derive(Debug, Clone, Encode, Decode)]
pub struct SolveStatus {
    pub timestamp: SystemTime,
    pub status:    Result<Statement, SolutionError>,

    pub cycles_count:  usize,
    pub absolute_time: f64,
}

pub struct Solution {
    pub problem: Problem,

    pub stack: Frame,

    local_rules: Vec<SharedRule>,
    target:      Target,
    pub answer:  Option<usize>,

    pub perf_stats: SolveStatus,
}

#[derive(Debug, Clone, Copy, Encode, Decode)]
pub enum SolutionError {
    StackOverflow,
    MaxSubproblemLevelExceed,
    NoConditions,
    NoSolutionsFound,
}

impl fmt::Display for SolutionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SolutionError::StackOverflow => write!(f, "StackOverflow"),
            SolutionError::MaxSubproblemLevelExceed => write!(f, "MaxSubproblemLevelExceed"),
            SolutionError::NoConditions => write!(f, "NoConditions"),
            SolutionError::NoSolutionsFound => write!(f, "NoSolutionsFound"),
        }
    }
}

impl Default for SolveStatus {
    fn default() -> SolveStatus {
        SolveStatus {
            timestamp:     SystemTime::now(),
            status:        Err(SolutionError::NoSolutionsFound),
            cycles_count:  0,
            absolute_time: 0.,
        }
    }
}

impl Solution {
    pub fn new(problem: Problem, rules: Arc<RulesEngine>, dumper: Dumper) -> Solution {
        Solution {
            stack: Frame::with_statements(
                rules.clone(),
                dumper.clone(),
                problem.conditions.iter().cloned(),
            ),

            local_rules: vec![],
            target: Target::try_from((*problem.target.statement).clone(), rules, dumper).unwrap(),

            answer: None,

            perf_stats: SolveStatus::default(),

            problem,
        }
    }

    pub fn answer(&self) -> Option<Arc<Statement>> {
        self.answer.map(|i| self.stack[i].statement.clone())
    }

    pub fn solve(&mut self) -> Result<(), SolutionError> {
        self.stack.dumper().subproblem_start(&self.problem);
        trace!(target: "subproblems", "Subproblem: {}, {:?}", self.target, self.problem.conditions);
        if self.problem.subproblem_level > MAX_SUBPROBLEM_LEVEL {
            return Err(SolutionError::MaxSubproblemLevelExceed);
        }

        let start = Instant::now();
        let result = self.solution_loop();

        self.perf_stats.status =
            result.map(|_| (*self.stack[self.answer.unwrap()].statement).clone());
        self.perf_stats.absolute_time = (start.elapsed().as_nanos() as f64) / 1000000.;
        self.stack.dumper().subproblem_end();
        result
    }

    fn solution_loop(&mut self) -> Result<(), SolutionError> {
        loop {
            self.perf_stats.cycles_count += 1;

            let index = self.stack.pick_condition()?;
            let level = self.stack[index].weight;
            trace!(target: "subproblem", "Subproblem level: {}", level);
            if level > MAX_LEVEL {
                return Err(SolutionError::NoSolutionsFound);
            }
            self.target.prepare_target(
                level,
                self.local_rules.clone(),
                &self.stack,
                &self.problem.target,
            );

            if !self.target.is_transform() {
                if let Some(simplified) = self.stack.transform(index) {
                    self.stack[index].replaced = true;
                    self.stack.add_condition(simplified).unwrap();
                    continue;
                } else {
                    self.stack[index].simplified = true;
                }
            }

            if let Some(suppose) = self.target.is_answer(&self.stack[index]) {
                if self.stack.suppose_proof(&suppose) {
                    trace!("Resolution: {}", suppose.resolution);
                    if self.stack[index] == suppose.resolution {
                        trace!("Equivalence");
                        self.answer = Some(index);
                    } else {
                        let _ = self.stack.add_condition(suppose.resolution.clone());
                        self.answer = Some(self.stack.find(&suppose.resolution.statement).unwrap());
                    }
                    trace!("Solved. Answer: {}", self.answer().unwrap());
                    return Ok(());
                }
            }
            if let Some(r) = self.stack[index].rule(
                (self.local_rules.len() + 1) | 0x80_00_00_00_00_00_00_00,
                (level + 1) as u64,
            ) {
                self.local_rules.push(r);
            }

            if !self.target.is_transform() {
                let statements = self.stack.next_statement(
                    self.local_rules.clone(),
                    index,
                    &self.problem.target,
                );
                if statements.is_empty() {
                    self.stack[index].weight += 1;
                }
                for s in statements {
                    trace!("{} => {}", self.stack[index], s);
                    self.stack.add_condition(s)?;
                }
            } else {
                self.stack[index].weight += 1;
            }
        }
    }
}

#[cfg(test)]
mod solution_tests {
    use std::sync::Arc;

    use crate::{
        problem::{parse_problem, Solution},
        rule::RulesEngine,
        statement::statement_with_vars,
        utils::Dumper,
    };

    #[test]
    fn check_answer_find_test() {
        let problem = parse_problem("problem {target find(x); x == 1;}");
        let rules = Arc::new(RulesEngine::default());
        let mut solution = Solution::new(problem, rules, Dumper::default());
        assert!(solution.solve().is_ok());
        assert_eq!(*solution.answer().unwrap(), statement_with_vars("x == 1"));
    }

    #[test]
    fn check_answer_proof_test() {
        let problem = parse_problem("problem { target proof(x > 0); x == 2; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solution = Solution::new(problem, rules, Dumper::default());
        assert!(solution.solve().is_ok());
        assert_eq!(*solution.answer().unwrap(), statement_with_vars("x == 2"));
    }
}

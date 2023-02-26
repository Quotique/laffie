use std::{
    fmt,
    sync::Arc,
    time::{Instant, SystemTime},
};

use bincode::{Decode, Encode};

use crate::{
    rule::{RulesEngine, SharedRule},
    statement::Statement,
    utils::{Dumper, DumperSink, VecDisplay},
    RuleId,
};

use super::{cache::ProblemsCache, frame::Frame, problem::Problem, target::Target};

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
    pub cache: Option<Arc<ProblemsCache>>,

    local_rules: Vec<SharedRule>,
    pub target:  Target,
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
                problem.subproblem_level,
            ),
            cache: None,

            local_rules: vec![],
            target: Target::try_from(
                (*problem.target.statement).clone(),
                rules,
                dumper,
                problem.subproblem_level,
            )
            .unwrap(),

            answer: None,

            perf_stats: SolveStatus::default(),

            problem,
        }
    }

    pub fn answer(&self) -> Option<Arc<Statement>> {
        self.answer.map(|i| self.stack[i].statement.clone())
    }

    pub fn solve(&mut self) -> Result<(), SolutionError> {
        self.cache = Some(Default::default());
        self.solve_subproblem(self.cache.as_ref().unwrap().clone())
    }

    pub fn solve_subproblem(&mut self, cache: Arc<ProblemsCache>) -> Result<(), SolutionError> {
        self.stack.dumper().subproblem_start(&self.problem);
        trace!(target: "subproblem", "Subproblem: {}, {}", self.target, VecDisplay(&self.problem.conditions));
        if self.problem.subproblem_level > MAX_SUBPROBLEM_LEVEL {
            return Err(SolutionError::MaxSubproblemLevelExceed);
        }

        let start = Instant::now();
        let result = self.solution_loop(cache);

        self.perf_stats.status =
            result.map(|_| (*self.stack[self.answer.unwrap()].statement).clone());
        self.perf_stats.absolute_time = (start.elapsed().as_nanos() as f64) / 1000000.;
        self.stack.dumper().subproblem_end();
        result
    }

    fn solution_loop(&mut self, cache: Arc<ProblemsCache>) -> Result<(), SolutionError> {
        loop {
            self.perf_stats.cycles_count += 1;

            let index = self.stack.pick_condition()?;
            let level = self.stack[index].weight;
            trace!(
                target: "subproblem",
                "[{}] Level: {} -> {}",
                self.problem.subproblem_level,
                level, self.stack[index]
            );
            if level > MAX_LEVEL {
                return Err(SolutionError::NoSolutionsFound);
            }
            self.target.prepare_target(
                level,
                self.local_rules.clone(),
                &self.stack,
                &self.problem.target,
                cache.clone(),
            );

            if !self.target.is_transform() {
                if let Some(simplified) = self.stack.transform(index, cache.clone()) {
                    self.stack[index].replaced = true;
                    self.stack.add_condition(simplified).unwrap();
                    continue;
                } else {
                    self.stack[index].simplified = true;
                }
            }

            if let Some(suppose) = self.target.is_answer(&self.stack[index]) {
                if self.stack.suppose_proof(&suppose, cache.clone()).is_some() {
                    trace!("Resolution: {}", suppose.resolution);
                    if self.stack[index] == suppose.resolution {
                        trace!("Equivalence");
                        self.answer = Some(index);
                    } else {
                        let _ = self.stack.add_condition(suppose.resolution.clone());
                        self.answer = Some(self.stack.find(&suppose.resolution.statement).unwrap());
                    }
                    trace!(
                        "Solved {}. Answer: {}",
                        self.problem.subproblem_level,
                        self.answer().unwrap()
                    );
                    return Ok(());
                }
            }
            if let Some(r) = self.stack[index].rule(
                RuleId::new(0x80_00_00_00_00_00_00_00, self.local_rules.len() as u64 + 1),
                (level + 1) as u64,
            ) {
                self.local_rules.push(r);
            }

            if !self.target.is_transform() {
                let statements = self.stack.next_statement(
                    &self.local_rules,
                    index,
                    &self.problem.target,
                    cache.clone(),
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

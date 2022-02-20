use super::{frame::Frame, problem::Problem, target::Target};
use crate::{
    rule::{RulesEngine, SharedRule},
    statement::Statement,
    utils::{Dumper, DumperSink},
};
use std::{fmt, sync::Arc, time::Instant};

use colored::*;

pub const MAX_SUBPROBLEM_LEVEL: usize = 10;
pub const MAX_LEVEL: usize = 20;

pub struct PerfStats {
    _problem_hash:   u64,
    cycles_count:    usize,
    _solution_depth: usize,
    absolute_time:   f64,
}

pub struct Solution {
    pub problem: Problem,

    stack: Frame,

    local_rules: Vec<SharedRule>,
    target:      Target,
    pub answer:  Option<usize>,

    pub perf_stats: PerfStats,
}

#[derive(Debug)]
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

impl PerfStats {
    pub fn new(problem: &Problem) -> PerfStats {
        PerfStats {
            _problem_hash:   problem.id,
            cycles_count:    0,
            _solution_depth: 0,
            absolute_time:   0.,
        }
    }
}

impl Solution {
    pub fn new(problem: Problem, rules: Arc<RulesEngine>, dumper: Dumper) -> Solution {
        Solution {
            stack: Frame::with_statements(
                rules.clone(),
                dumper,
                problem.conditions.iter().cloned(),
            ),

            local_rules: vec![],
            target: Target::try_from((*problem.target.statement).clone(), rules).unwrap(),

            answer: None,

            perf_stats: PerfStats::new(&problem),

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

impl fmt::Display for Solution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(a) = self.answer.as_ref() {
            let mut trace: Vec<(usize, Vec<usize>)> = vec![];

            fn visitor(stack: &Frame, id: usize, trace: &mut Vec<(usize, Vec<usize>)>) {
                trace.push((id, stack[id].parents.clone()));
                for i in stack[id].parents.iter() {
                    visitor(stack, *i, trace);
                }
            }

            visitor(&self.stack, *a, &mut trace);

            while let Some(t) = trace.pop() {
                writeln!(f)?;
                for p in t.1.iter() {
                    writeln!(f, "{},", self.stack[*p].statement.to_string().underline())?;
                }
                writeln!(
                    f,
                    "{}",
                    self.stack[t.0].statement.to_string().bold().yellow()
                )?;
            }
            writeln!(
                f,
                "{} {}",
                "SOLVED!".green(),
                format!(
                    "[{} cycles, {}ms]",
                    self.perf_stats.cycles_count, self.perf_stats.absolute_time
                )
                .yellow()
            )
        } else {
            writeln!(f)?;
            writeln!(f, "{}", "NOT SOLVED!".bold().blink().red())
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

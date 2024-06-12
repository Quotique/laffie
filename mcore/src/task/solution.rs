use std::{
    fmt,
    rc::Rc,
    sync::Arc,
    time::{Instant, SystemTime},
};

use bincode::{Decode, Encode};

use crate::{
    rule::{RulesEngine, SharedRule},
    term::Term,
    utils::{Dumper, DumperSink, VecDisplay},
    RuleId,
};

use super::{cache::TasksCache, frame::Frame, purpose::Purpose, Task};

pub const MAX_SUBTASK_LEVEL: usize = 10;
pub const MAX_LEVEL: usize = 20;

#[derive(Debug, Clone, Encode, Decode)]
pub struct SolveStatus {
    pub timestamp: SystemTime,
    pub status:    Result<Term, SolutionError>,

    pub cycles_count:  usize,
    pub absolute_time: f64,
}

pub struct Solution {
    pub task: Task,

    pub stack: Frame,
    pub cache: Option<Arc<TasksCache>>,

    local_rules: Vec<SharedRule>,
    pub purpose: Purpose,
    pub answer:  Option<usize>,

    pub perf_stats: SolveStatus,
}

#[derive(Debug, Clone, Copy, Encode, Decode)]
pub enum SolutionError {
    StackOverflow,
    MaxSubtaskLevelExceed,
    NoConditions,
    NoSolutionsFound,
}

impl fmt::Display for SolutionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SolutionError::StackOverflow => write!(f, "StackOverflow"),
            SolutionError::MaxSubtaskLevelExceed => write!(f, "MaxSubtaskLevelExceed"),
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
    pub fn new(task: Task, rules: Arc<RulesEngine>, dumper: Dumper) -> Solution {
        Solution {
            stack: Frame::with_terms(
                rules.clone(),
                dumper.clone(),
                task.conditions.iter().cloned(),
                task.subtask_level,
            ),
            cache: None,

            local_rules: vec![],
            purpose: Purpose::try_from(
                (*task.purpose.term).clone(),
                rules,
                dumper,
                task.subtask_level,
            )
            .unwrap(),

            answer: None,

            perf_stats: SolveStatus::default(),

            task,
        }
    }

    pub fn answer(&self) -> Option<Rc<Term>> {
        self.answer.map(|i| self.stack[i].term.clone())
    }

    pub fn solve(&mut self) -> Result<(), SolutionError> {
        self.cache = Some(Default::default());
        self.solve_subtask(self.cache.as_ref().unwrap().clone())
    }

    pub fn solve_subtask(&mut self, cache: Arc<TasksCache>) -> Result<(), SolutionError> {
        self.stack.dumper().subtask_start(&self.task);
        trace!(target: "subtask", "Subtask: {}, {}", self.purpose, VecDisplay(&self.task.conditions));
        if self.task.subtask_level > MAX_SUBTASK_LEVEL {
            return Err(SolutionError::MaxSubtaskLevelExceed);
        }

        let start = Instant::now();
        let result = self.solution_loop(cache);

        self.perf_stats.status = result.map(|_| (*self.stack[self.answer.unwrap()].term).clone());
        self.perf_stats.absolute_time = (start.elapsed().as_nanos() as f64) / 1000000.;
        self.stack.dumper().subtask_end(&self.perf_stats);
        result
    }

    fn solution_loop(&mut self, cache: Arc<TasksCache>) -> Result<(), SolutionError> {
        loop {
            self.perf_stats.cycles_count += 1;

            let index = self.stack.pick_condition()?;
            let level = self.stack[index].weight;
            trace!(
                target: "subtask",
                "[{}] Level: {} -> {}",
                self.task.subtask_level,
                level, self.stack[index]
            );
            if level > MAX_LEVEL {
                return Err(SolutionError::NoSolutionsFound);
            }
            self.purpose.prepare_purpose(
                level,
                self.local_rules.clone(),
                &self.stack,
                &self.task.purpose,
                cache.clone(),
            );

            if !self.purpose.is_transform() {
                if let Some(simplified) = self.stack.transform(index, cache.clone()) {
                    self.stack[index].replaced = true;
                    self.stack.add_condition(simplified).unwrap();
                    continue;
                } else {
                    self.stack[index].simplified = true;
                }
            }

            if let Some(suppose) = self.purpose.is_answer(&self.stack[index]) {
                if self.stack.suppose_proof(&suppose, cache.clone()).is_some() {
                    trace!("Resolution: {}", suppose.resolution);
                    if self.stack[index] == suppose.resolution {
                        trace!("Equivalence");
                        self.answer = Some(index);
                    } else {
                        let _ = self.stack.add_condition(suppose.resolution.clone());
                        self.answer = Some(self.stack.find(&suppose.resolution.term).unwrap());
                    }
                    trace!(
                        "Solved {}. Answer: {}",
                        self.task.subtask_level,
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

            if !self.purpose.is_transform() {
                let terms = self.stack.next_term(
                    &self.local_rules,
                    index,
                    &self.task.purpose,
                    cache.clone(),
                );
                if terms.is_empty() {
                    self.stack[index].weight += 1;
                }
                for s in terms {
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
        rule::RulesEngine,
        task::{parse_task, Solution},
        term::term_with_vars,
        utils::Dumper,
    };

    #[test]
    fn check_answer_find_test() {
        let task = parse_task("task {purpose find(x); x == 1;}");
        let rules = Arc::new(RulesEngine::default());
        let mut solution = Solution::new(task, rules, Dumper::default());
        assert!(solution.solve().is_ok());
        assert_eq!(*solution.answer().unwrap(), term_with_vars("x == 1"));
    }

    #[test]
    fn check_answer_proof_test() {
        let task = parse_task("task { purpose proof(x > 0); x == 2; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solution = Solution::new(task, rules, Dumper::default());
        assert!(solution.solve().is_ok());
        assert!(solution.answer().is_some());
    }
}

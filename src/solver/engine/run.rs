use std::{
    collections::{HashMap, hash_map::Entry},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use super::{SharedSolution, Solution, SolutionStatus, SolveError, TracerHub};
use crate::{
    task::{Goal, Task},
    term::TermBuf,
};

/// Cycle budget; past it a run ends with `ExecutionDeadline`.
pub const EXECUTION_DEADLINE_DEFAULT: usize = 100_000;

/// Wall-clock budget, effectively unlimited.
pub const TIME_LIMIT_DEFAULT: Duration = Duration::from_secs(24 * 60 * 60);

/// Every clone shares one flag, so any thread can stop the run.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// What stops a run: cycle budget, wall clock, cancellation.
#[derive(Clone)]
pub struct Limits {
    execution_deadline: usize,
    deadline_at:        Instant,
    cancel:             CancelToken,
}

impl Limits {
    /// The returned token shares the cancel flag.
    pub fn init(execution_deadline: usize, time_limit: Duration) -> (Self, CancelToken) {
        let cancel = CancelToken::new();
        let limits = Self {
            execution_deadline,
            deadline_at: Instant::now() + time_limit,
            cancel: cancel.clone(),
        };
        (limits, cancel)
    }

    /// Cancellation is checked before the budgets.
    pub(super) fn check(&self, cycle: usize) -> Result<(), SolveError> {
        if self.cancel.is_cancelled() {
            return Err(SolveError::Canceled);
        }
        if cycle > self.execution_deadline {
            return Err(SolveError::ExecutionDeadline);
        }
        if Instant::now() >= self.deadline_at {
            return Err(SolveError::TimeDeadline);
        }
        Ok(())
    }
}

/// Shared by every frame of one `solve`. A copy per subtask would change
/// the cycle count and the cache hits.
pub(super) struct Run {
    pub(super) limits: Limits,
    pub(super) cycle:  usize,
    pub(super) cache:  HashMap<CacheKey, SharedSolution>,
    pub(super) tracer: TracerHub,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) enum CacheKey {
    Goal(Goal),
    SolveBlock(TermBuf),
}

pub(super) enum SubtaskEntry {
    Occupied(SharedSolution),
    Vacant(CacheSlot),
}

#[must_use]
pub(super) struct CacheSlot(CacheKey);

impl Run {
    pub(super) fn cycle(&self) -> usize {
        self.cycle
    }

    pub(super) fn subtask_entry(&mut self, key: CacheKey, goal: &Goal) -> SubtaskEntry {
        match self.cache.entry(key) {
            Entry::Occupied(e) => SubtaskEntry::Occupied(e.get().clone()),
            Entry::Vacant(e) => {
                let key = e.key().clone();
                e.insert(Solution::new(Task::from_goal(goal.clone())).into());
                SubtaskEntry::Vacant(CacheSlot(key))
            }
        }
    }

    pub(super) fn fill(&mut self, slot: CacheSlot, solution: SharedSolution) -> SharedSolution {
        if let SolutionStatus::Err(SolveError::MaxSubtaskLevelExceed) = solution.status {
            self.cache.remove(&slot.0);
        } else {
            self.cache.insert(slot.0, solution.clone());
        }
        solution
    }

    /// Counts the cycle before checking the limits.
    pub(super) fn begin_cycle(&mut self) -> Result<(), SolveError> {
        self.cycle += 1;
        self.limits.check(self.cycle)
    }
}

#[cfg(test)]
mod cache_tests {
    use std::time::Duration;

    use super::{
        CacheKey, Limits, Run, SharedSolution, Solution, SolutionStatus, SolveError, SubtaskEntry,
        Task,
    };
    use crate::{
        engine::TracerHub,
        task::Goal,
        term::{TermBuf, term_with_vars},
    };

    fn run() -> Run {
        Run {
            limits: Limits::init(usize::MAX, Duration::from_secs(60)).0,
            cycle:  0,
            cache:  Default::default(),
            tracer: TracerHub::default(),
        }
    }

    fn goal(src: &'static str) -> Goal {
        Goal::parse(term_with_vars(src)).expect("a goal")
    }

    #[test]
    fn a_second_reservation_of_one_key_hits_the_placeholder() {
        let mut run = run();
        let g = goal("prove(x > 0)");

        let SubtaskEntry::Vacant(_slot) = run.subtask_entry(CacheKey::Goal(g.clone()), &g) else {
            panic!("the first reservation takes the key");
        };
        let SubtaskEntry::Occupied(placeholder) = run.subtask_entry(CacheKey::Goal(g.clone()), &g)
        else {
            panic!("the second must hit, not take the key again");
        };
        // No answer, so the caller drops its hypothesis instead of looping.
        assert!(placeholder.answer().is_none());
    }

    #[test]
    fn a_depth_failure_releases_the_key() {
        let mut run = run();
        let g = goal("prove(x > 0)");
        let SubtaskEntry::Vacant(slot) = run.subtask_entry(CacheKey::Goal(g.clone()), &g) else {
            panic!("reserved");
        };

        let mut failed = Solution::new(Task::from_goal(g.clone()));
        failed.status = SolutionStatus::Err(SolveError::MaxSubtaskLevelExceed);
        run.fill(slot, SharedSolution::new(failed));

        // Met higher up, the same subtask has to be solvable afresh.
        assert!(matches!(
            run.subtask_entry(CacheKey::Goal(g.clone()), &g),
            SubtaskEntry::Vacant(_)
        ));
    }

    #[test]
    fn any_other_failure_stays_cached() {
        let mut run = run();
        let g = goal("prove(x > 0)");
        let SubtaskEntry::Vacant(slot) = run.subtask_entry(CacheKey::Goal(g.clone()), &g) else {
            panic!("reserved");
        };

        let mut failed = Solution::new(Task::from_goal(g.clone()));
        failed.status = SolutionStatus::Err(SolveError::NoSolutionsFound);
        run.fill(slot, SharedSolution::new(failed));

        let SubtaskEntry::Occupied(cached) = run.subtask_entry(CacheKey::Goal(g.clone()), &g)
        else {
            panic!("a settled subtask stays settled");
        };
        assert!(matches!(
            cached.status,
            SolutionStatus::Err(SolveError::NoSolutionsFound)
        ));
    }

    #[test]
    fn a_goal_and_a_solve_call_are_different_keys() {
        let mut run = run();
        let g = goal("find(x)");
        let call = TermBuf::symbol("solve").arg(term_with_vars("find(x)"));

        let SubtaskEntry::Vacant(_) = run.subtask_entry(CacheKey::Goal(g.clone()), &g) else {
            panic!("reserved");
        };
        assert!(
            matches!(
                run.subtask_entry(CacheKey::SolveBlock(call), &g),
                SubtaskEntry::Vacant(_)
            ),
            "a solve(...) call must not collide with the goal inside it"
        );
    }
}

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    error,
    ops::{Index, IndexMut},
    sync::Arc,
};

use derive_more::Display;
use itertools::Itertools;

use super::{TermInference, TermProps};
use crate::{
    CompactString, NormLevel,
    rule::{Level, RuleId},
    task::{Goal, Task, TaskBuilder},
    term::{Atom, SharedTerm, Term, TermBuf, answer, match_term},
};

pub const STACK_SIZE: usize = 2048;

pub type SharedSolution = Arc<Solution>;
pub type TermIdx = usize;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum SolveError {
    StackOverflow,
    MaxSubtaskLevelExceed,
    NoConditions,
    NoSolutionsFound,
    ExecutionDeadline,
    Canceled,
    TimeDeadline,
    Internal,
}

#[derive(Debug, Display, Default, Clone, Copy, PartialEq, Eq)]
pub enum SolutionStatus {
    #[default]
    NotDone,
    Answer(usize),
    Err(SolveError),
}

#[derive(Debug)]
pub struct Solution {
    pub task: Task,

    pub status:      SolutionStatus,
    pub start_cycle: usize,
    pub end_cycle:   usize,

    pub main_index:       HashMap<SharedTerm, usize>,
    pub goal_index:       HashMap<SharedTerm, usize>,
    pub terms:            Vec<TermProps>,
    pub find_bindings:    HashMap<TermBuf, TermIdx>,
    unproven_terms_count: usize,

    /// Names of variables declared known (`v is known`) among the givens.
    /// The truth of `_ is known` is evaluated against this set instead of a
    /// per-atom flag, so term identity stays independent of known-ness.
    pub known_vars: HashSet<CompactString>,

    /// Min-heap of proven, pickable terms keyed by `(level, id)` so that
    /// `pick_next` is O(log n) instead of a full scan. Holds stale entries
    /// (replaced terms, superseded levels) that are discarded lazily on peek.
    agenda: BinaryHeap<Reverse<(Level, TermIdx)>>,
}

impl Solution {
    /// Infallible: `task` guarantees a well-formed goal.
    pub fn new(task: Task) -> Self {
        Self::seeded(task, Vec::new(), HashSet::new())
    }

    pub(crate) fn subtask(
        &self,
        goal: Goal,
        blocked_rules: HashSet<RuleId>,
        extra: Vec<SharedTerm>,
    ) -> Self {
        let inherited: Vec<TermProps> = self
            .terms
            .iter()
            .filter(|x| x.is_proven())
            .filter(|x| !(x.filters.is_goal() || answer::marked(x.term.term()).is_some()))
            .map(|x| {
                let mut props = x.clone();
                props.filters.level = 0.into();
                props.inference = TermInference::Condition;
                props
            })
            .collect();

        let task = TaskBuilder::from_goal(goal)
            .with_conditions(inherited.iter().map(|x| x.term.clone()).chain(extra))
            .with_level(self.task.subtask_level + 1)
            .build();
        Self::seeded(task, inherited, blocked_rules)
    }

    /// INVARIANT: `history` holds the props of `task.givens`, same terms in
    /// the same order. [`Solution::subtask`] builds both from one list.
    fn seeded(task: Task, history: Vec<TermProps>, blocked_rules: HashSet<RuleId>) -> Self {
        let known_vars = collect_known_set(&task.givens);
        let mut solution = Self {
            task,
            start_cycle: Default::default(),
            end_cycle: Default::default(),
            main_index: Default::default(),
            goal_index: Default::default(),
            terms: Default::default(),
            find_bindings: Default::default(),
            status: Default::default(),
            unproven_terms_count: Default::default(),
            agenda: Default::default(),
            known_vars,
        };
        let conditions = solution.task.givens.clone();
        let mut history = history.into_iter();
        for term in conditions {
            let props = history.next().unwrap_or_else(|| TermProps::from(term));
            let _ = solution.add_term(props);
        }
        let mut goal_term = TermProps::from(solution.goal().subject().to_owned());
        // The search tells goal terms from derived ones by this flag alone.
        goal_term.filters.mark_goal();
        // The task's blocked rules land on the goal term the search starts from.
        goal_term.filters.blocked_rules = blocked_rules;
        let goal_buf = Arc::make_mut(&mut goal_term.term);
        // Canonicalize the goal (commutative arg order etc.) so the syntactic
        // match in `check_prove_answer` sees derived terms — which are always
        // normalized — in the same shape as the goal.
        goal_buf.term_mut().normalize(NormLevel::Full);
        let _ = solution.add_term(goal_term);

        trace!(target: "subtask", "Subtask: {}, [{}]",
            solution.goal(), solution.task.givens.iter().format(", ")
        );
        solution
    }

    #[inline]
    pub fn goal(&self) -> &Goal {
        self.task.goal()
    }

    #[inline]
    pub fn cycles(&self) -> usize {
        self.end_cycle - self.start_cycle
    }

    pub fn add_term(&mut self, mut term: TermProps) -> Result<TermIdx, SolveError> {
        if self.terms.len() - self.unproven_terms_count + 1 > STACK_SIZE {
            return Err(SolveError::StackOverflow);
        }
        let id = self.terms.len();
        term.id = id;
        term.finalize_proven();

        if !term.is_proven() {
            self.unproven_terms_count += 1;
            self.terms.push(term);
            return Ok(id);
        }

        let index = if term.filters.is_goal() {
            &mut self.goal_index
        } else {
            &mut self.main_index
        };
        if let Some(id) = index.get(&term.term) {
            return Ok(*id);
        }
        index.insert(term.term.clone(), id);

        self.agenda.push(Reverse((term.filters.level, id)));
        self.terms.push(term);
        Ok(id)
    }

    /// Re-inserts a term into the agenda after its level changed; the previous
    /// entry is left to be discarded lazily by `pick_next`.
    pub fn requeue(&mut self, index: TermIdx) {
        self.agenda
            .push(Reverse((self.terms[index].filters.level, index)));
    }

    pub fn pick_next(&mut self) -> Option<TermIdx> {
        while let Some(&Reverse((level, id))) = self.agenda.peek() {
            let term = &self.terms[id];
            if !term.filters.is_replaced() && term.filters.level == level {
                return Some(id);
            }
            self.agenda.pop();
        }
        None
    }

    pub fn pick_goal_term(&self) -> Option<TermIdx> {
        self.terms
            .iter()
            .filter(|x| x.is_proven())
            .filter(|x| !x.filters.is_replaced() && x.filters.is_goal())
            .min_by_key(|x| x.filters.level)
            .map(|x| x.id)
    }

    #[inline]
    pub fn answer(&self) -> Option<SharedTerm> {
        if let SolutionStatus::Answer(i) = self.status {
            return Some(self.terms[i].term.clone());
        }
        None
    }

    pub fn validate_answer(&self) -> bool {
        if self.task.possible_answers.is_empty() {
            return true;
        }

        if let Some(answer) = self.answer() {
            if self
                .task
                .possible_answers
                .iter()
                .any(|x| x == answer.as_ref())
            {
                return true;
            }
            // TODO: есть проблема с неправильным преобразованием дерева, что приводит к
            // некорректному прямому сравнению дерева.
            return self
                .task
                .possible_answers
                .iter()
                .any(|x| x.to_string() == answer.to_string());
        }
        false
    }
}

impl Index<TermIdx> for Solution {
    type Output = TermProps;

    fn index(&self, index: TermIdx) -> &Self::Output {
        &self.terms[index]
    }
}

impl IndexMut<TermIdx> for Solution {
    fn index_mut(&mut self, index: TermIdx) -> &mut Self::Output {
        &mut self.terms[index]
    }
}

impl error::Error for SolveError {}

/// Collect names declared known via a `V is known` given (V a bare variable).
fn collect_known_set(givens: &[SharedTerm]) -> HashSet<CompactString> {
    let mut set = HashSet::new();
    for g in givens {
        let Some((lhs,)) = match_term!(g.term(), "is"(lhs, "known")) else {
            continue;
        };
        if let Atom::Variable(v) = lhs.data() {
            set.insert(v.as_ref().clone());
        }
    }
    set
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::Solution;
    use crate::{
        rule::RuleId,
        task::{Goal, parse_task},
        term::{TermBuf, term_with_vars},
    };

    #[test]
    fn blocked_rules_reach_the_goal_term_the_search_starts_from() {
        // Or the rule that blocked itself fires again inside the subtask.
        let blocked = RuleId::new(0, 7);
        let parent = Solution::new(parse_task("task { goal find(x); x == 1; }"));
        let goal = Goal::parse(TermBuf::symbol("transform").arg(term_with_vars("1 + 2")))
            .expect("transform(1 + 2) is a goal");

        let child = parent.subtask(goal, HashSet::from([blocked]), Vec::new());

        let goal_term = child
            .terms
            .iter()
            .find(|t| t.filters.is_goal())
            .expect("the goal term is in the solution");
        assert!(goal_term.filters.blocked_rules.contains(&blocked));
    }

    fn is_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_send_sync_solution() {
        is_send_sync::<Solution>();
    }

    #[test]
    fn known_vars_from_givens() {
        let task = parse_task("task { goal find(x); a*x == b; a is known; b is known; }");
        let solution = Solution::new(task);

        assert!(solution.known_vars.contains("a"));
        assert!(solution.known_vars.contains("b"));
        assert!(!solution.known_vars.contains("x"));
    }
}

use std::{
    collections::{HashMap, HashSet},
    error,
    ops::{Index, IndexMut},
    sync::Arc,
};

use bincode::{Decode, Encode};
use derive_more::Display;
use itertools::Itertools;

use super::{Goal, Task, TermProps};
use crate::{
    CompactString,
    term::{Atom, SharedTerm, Term, TermBuf, TermMut, match_term},
};

pub const STACK_SIZE: usize = 2048;

pub type SharedSolution = Arc<Solution>;
pub type TermIdx = usize;

#[derive(Debug, Display, Clone, Copy, Encode, Decode, PartialEq, Eq)]
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

#[derive(Debug, Display, Default, Clone, Copy, Encode, Decode, PartialEq, Eq)]
pub enum SolutionStatus {
    #[default]
    NotDone,
    Answer(usize),
    Err(SolveError),
}

#[derive(Debug)]
pub struct Solution {
    pub task: Task,
    pub goal: Goal,

    pub status:      SolutionStatus,
    pub start_cycle: usize,
    pub end_cycle:   usize,

    pub main_index:       HashMap<SharedTerm, usize>,
    pub goal_index:       HashMap<SharedTerm, usize>,
    pub terms:            Vec<TermProps>,
    pub find_bindings:    HashMap<TermBuf, TermIdx>,
    unproven_terms_count: usize,
}

impl Solution {
    pub fn new(task: Task) -> Self {
        let mut goal = Goal::try_from((*task.goal.term).clone()).unwrap();
        // Goal::try_from rebuilds TermProps from a bare TermBuf and drops the
        // task.goal.filters payload. Carry blocked_rules across so that a
        // `block(rule_id)` set on the parent term survives into the subtask
        // (e.g. a transform-subtask whose goal was produced by a rule with
        // `block(...)` must still skip the blocked rule).
        let inherited_blocked = task.goal.filters.blocked_rules.clone();
        if !inherited_blocked.is_empty() {
            match &mut goal {
                Goal::Find(g) => g.term.filters.blocked_rules.extend(inherited_blocked),
                Goal::Prove(t) | Goal::Transform(t) => {
                    t.filters.blocked_rules.extend(inherited_blocked);
                }
            }
        }

        let mut solution = Self {
            task,
            goal,
            start_cycle: Default::default(),
            end_cycle: Default::default(),
            main_index: Default::default(),
            goal_index: Default::default(),
            terms: Default::default(),
            find_bindings: Default::default(),
            status: Default::default(),
            unproven_terms_count: Default::default(),
        };
        let known = collect_known_set(&solution.task.givens);
        let conditions = solution.task.givens.clone();
        for mut i in conditions.into_iter() {
            stamp_known(&mut Arc::make_mut(&mut i.term).term_mut(), &known);
            let _ = solution.add_term(i);
        }
        let mut goal_term = solution.goal.term().clone();
        stamp_known(&mut Arc::make_mut(&mut goal_term.term).term_mut(), &known);
        let _ = solution.add_term(goal_term);

        trace!(target: "subtask", "Subtask: {}, [{}]",
            solution.goal, solution.task.givens.iter().format(", ")
        );
        solution
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

        if !term.inference.is_proven() {
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

        self.terms.push(term);
        Ok(id)
    }

    pub fn pick_next(&self) -> Option<TermIdx> {
        self.terms
            .iter()
            .filter(|x| x.inference.is_proven())
            .filter(|x| !x.filters.is_replaced())
            .min_by_key(|x| x.filters.level)
            .map(|x| x.id)
    }

    pub fn pick_goal_term(&self) -> Option<TermIdx> {
        self.terms
            .iter()
            .filter(|x| x.inference.is_proven())
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
fn collect_known_set(givens: &[TermProps]) -> HashSet<CompactString> {
    let mut set = HashSet::new();
    for g in givens {
        let Some((lhs,)) = match_term!(g.term.term(), "is"(lhs, "known")) else {
            continue;
        };
        if let Atom::Variable(v) = lhs.data() {
            set.insert(v.as_ref().clone());
        }
    }
    set
}

/// Stamp `known = true` on every variable whose name was declared known.
fn stamp_known(term: &mut TermMut<'_>, known: &HashSet<CompactString>) {
    if let Atom::Variable(v) = term.data_mut() &&
        known.contains(v.as_ref())
    {
        v.known = true;
    }
    for mut child in term.iter_mut() {
        stamp_known(&mut child, known);
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::Solution;
    use crate::{
        task::parse_task,
        term::{Atom, Term, TermRef},
    };

    fn is_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_send_sync_solution() {
        is_send_sync::<Solution>();
    }

    fn collect_var_known(term: TermRef<'_>, out: &mut HashMap<String, bool>) {
        if let Atom::Variable(v) = term.data() {
            *out.entry(v.as_str().to_string()).or_insert(false) |= v.known;
        }
        for child in term.args_iter() {
            collect_var_known(child, out);
        }
    }

    #[test]
    fn stamps_known_from_givens() {
        let task = parse_task("task { goal find(x); a*x == b; a is known; b is known; }");
        let solution = Solution::new(task);

        let mut status = HashMap::new();
        for tp in &solution.terms {
            collect_var_known(tp.term.term(), &mut status);
        }

        assert_eq!(status.get("a"), Some(&true));
        assert_eq!(status.get("b"), Some(&true));
        assert_eq!(status.get("x"), Some(&false));
    }
}

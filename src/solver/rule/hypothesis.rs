use std::{collections::HashSet, fmt};

use itertools::Itertools;

use super::{ApplyRule, RuleId, SharedRule, TermFilters};
use crate::term::{Param, ParamSubstitution, TermBuf};

/// A hypothesis that may contain free (unbound) parameters (non-ground).
/// Trivially converts to [`GroundedHypothesis`] when `free_params` is empty.
#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub rule:       SharedRule,
    pub resolution: TermBuf,

    pub free_params: HashSet<Param>,

    pub params:        ParamSubstitution,
    pub requirements:  Vec<TermBuf>,
    pub blocked_rules: Vec<RuleId>,
}

/// A fully instantiated hypothesis with all parameters bound (ground instance).
#[derive(Debug, Clone)]
pub struct GroundedHypothesis {
    pub rule:       SharedRule,
    pub resolution: TermBuf,

    pub params:        ParamSubstitution,
    pub requirements:  Vec<TermBuf>,
    pub blocked_rules: Vec<RuleId>,
}

pub enum HypothesisIterator {
    Empty,
    Iter(std::vec::IntoIter<Hypothesis>),
}

impl Hypothesis {
    /// Returns `true` if all parameters are bound (ground hypothesis).
    pub fn is_grounded(&self) -> bool {
        self.free_params.is_empty()
    }

    /// Grounds the hypothesis into concrete instances.
    /// Trivial conversion when no free parameters exist.
    /// TODO: delegate to a `find(free_params)` subtask when free params are
    /// present.
    pub fn ground(self) -> Vec<GroundedHypothesis> {
        if self.is_grounded() {
            vec![GroundedHypothesis {
                rule:          self.rule,
                resolution:    self.resolution,
                params:        self.params,
                requirements:  self.requirements,
                blocked_rules: self.blocked_rules,
            }]
        } else {
            // TODO: delegate to find(free_params) subtask with constraints
            trace!(
                target: "rule_selection",
                "hypothesis has free params {:?}, grounding not yet implemented",
                self.free_params
            );
            vec![]
        }
    }
}

impl GroundedHypothesis {
    #[inline]
    pub fn rule(&self) -> SharedRule {
        self.rule.clone()
    }
}

impl HypothesisIterator {
    pub fn new(rule: SharedRule, term: &TermBuf, filters: &TermFilters, goal: &TermBuf) -> Self {
        let hypothesis = match rule.apply(term, filters, goal) {
            Ok(x) => x,
            Err(e) => {
                trace!(target: "rule_selection", "rule {rule} not applied to term {term}: {e:?}");
                return Self::empty();
            }
        };

        Self::Iter(hypothesis.into_iter())
    }

    pub fn empty() -> Self {
        Self::Empty
    }
}

impl Iterator for HypothesisIterator {
    type Item = Hypothesis;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::Iter(i) => i.next(),
        }
    }
}

impl fmt::Display for Hypothesis {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.free_params.is_empty() {
            write!(
                f,
                "[{}] => {}",
                self.requirements.iter().format(", "),
                self.resolution,
            )
        } else {
            write!(
                f,
                "[for {}: {}] => {}",
                self.free_params.iter().format(", "),
                self.requirements.iter().format(", "),
                self.resolution,
            )
        }
    }
}

impl fmt::Display for GroundedHypothesis {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{}] => {}",
            self.requirements.iter().format(", "),
            self.resolution,
        )
    }
}

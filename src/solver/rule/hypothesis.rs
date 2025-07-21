use std::fmt;

use itertools::Itertools;

use super::{ApplyRule, RuleId, SharedRule, TermFilters};
use crate::term::{ParamsMapping, Term};

#[derive(Debug)]
pub struct Hypothesis {
    pub rule:       SharedRule,
    pub resolution: Term,

    pub params:        ParamsMapping,
    pub requirements:  Vec<Term>,
    pub blocked_rules: Vec<RuleId>,
}

pub enum HypothesisIterator {
    Empty,
    Iter(std::vec::IntoIter<Hypothesis>),
}

impl Hypothesis {
    #[inline]
    pub fn rule(&self) -> SharedRule {
        self.rule.clone()
    }
}

impl HypothesisIterator {
    pub fn new(rule: SharedRule, term: &Term, filters: &TermFilters, purpose: &Term) -> Self {
        let hypothesis = match rule.apply(term, filters, purpose) {
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
        write!(
            f,
            "[{}] => {}",
            self.requirements.iter().format(", "),
            self.resolution,
        )
    }
}

use std::{collections::HashSet, fmt};

use itertools::Itertools;

use super::{ApplyRule, RuleId, SharedRule, TermFilters};
use crate::{
    NormalizationLevel,
    term::{Param, ParamSubstitution, Substitute, Term as _, TermBuf, match_term},
};

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
    ///
    /// - No free parameters → trivial conversion.
    /// - `<param> in set(...)` generators → cartesian product of set elements.
    /// - No generators found → returns empty (TODO: delegate to `find`
    ///   subtask).
    pub fn ground(mut self) -> Vec<GroundedHypothesis> {
        if self.is_grounded() {
            return vec![GroundedHypothesis {
                rule:          self.rule,
                resolution:    self.resolution,
                params:        self.params,
                requirements:  self.requirements,
                blocked_rules: self.blocked_rules,
            }];
        }

        // Normalize requirements so that e.g. `divisors(6)` becomes `set(...)`.
        self.requirements = self
            .requirements
            .drain(..)
            .map(|r| r.normalize(NormalizationLevel::max()))
            .collect();

        // Extract generator requirements; remaining stay in self.requirements
        let generators = self.extract_generators();

        if generators.is_empty() {
            trace!(
                target: "rule_selection",
                "hypothesis has free params {:?}, no set generator found",
                self.free_params
            );
            return vec![];
        }

        generators
            .into_iter()
            .map(|(param, elements)| {
                elements
                    .into_iter()
                    .map(move |e| (param.clone(), e))
                    .collect::<Vec<_>>()
            })
            .multi_cartesian_product()
            .map(|combo| {
                let mut subst = ParamSubstitution::default();
                for (param, element) in combo {
                    subst.params.insert(param, element);
                }

                let resolution = self.resolution.clone().substituted(&subst);
                let requirements = self
                    .requirements
                    .iter()
                    .map(|r| r.clone().substituted(&subst))
                    .collect();

                let mut params = self.params.clone();
                params.params.extend(subst.params);

                GroundedHypothesis {
                    rule: self.rule.clone(),
                    resolution,
                    params,
                    requirements,
                    blocked_rules: self.blocked_rules.clone(),
                }
            })
            .collect()
    }

    /// Extracts `<param> in set(...)` requirements as generators.
    /// Matched requirements are removed from `self.requirements`.
    fn extract_generators(&mut self) -> Vec<(Param, Vec<TermBuf>)> {
        self.requirements
            .extract_if(.., |req| {
                match_term!(req.term(), "in"(lhs, "set"(s)))
                    .and_then(|(lhs, _)| lhs.data().param().map(|_| ()))
                    .is_some()
            })
            .filter_map(|req| {
                let (lhs, set_node) = match_term!(req.term(), "in"(lhs, "set"(s)))?;
                let param = lhs.data().param()?.clone();
                let elements = set_node.args_iter().map(|a| a.to_owned()).collect();
                Some((param, elements))
            })
            .collect()
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

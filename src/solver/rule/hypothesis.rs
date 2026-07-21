use std::{collections::HashSet, fmt};

use itertools::{Either, Itertools};

use super::{ApplyRule, RuleId, SharedRule, TermFilters};
use crate::{
    NormLevel,
    term::{
        Atom, Param, ParamSubstitution, Substitute, Term as _, TermBuf, TermPath, Truth, match_term,
    },
};

/// Hypothesis with possibly free params; grounds via [`Hypothesis::ground`].
#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub rule:       SharedRule,
    pub resolution: TermBuf,

    pub free_params: HashSet<Param>,

    pub params:        ParamSubstitution,
    pub requirements:  Vec<TermBuf>,
    pub blocked_rules: Vec<RuleId>,

    /// Match position in the parent term
    pub pos: TermPath,
}

/// Fully instantiated hypothesis (no free params).
#[derive(Debug, Clone)]
pub struct GroundedHypothesis {
    pub rule:       SharedRule,
    pub resolution: TermBuf,

    pub params:        ParamSubstitution,
    pub requirements:  Vec<TermBuf>,
    pub blocked_rules: Vec<RuleId>,

    /// Match position carried from [`Hypothesis`] for context resolution.
    pub pos: TermPath,
}

pub enum HypothesisIterator {
    Empty,
    Iter(std::vec::IntoIter<Hypothesis>),
}

impl Hypothesis {
    /// `true` iff all params bound.
    pub fn is_grounded(&self) -> bool {
        self.free_params.is_empty()
    }

    /// Grounds via: bind `==`, then cartesian over `param in set(...)`
    /// generators, then re-bind `==` per combo. Lazy — the product is walked on
    /// demand, never fully materialized.
    pub fn ground(mut self) -> impl Iterator<Item = GroundedHypothesis> {
        self.bind_equality_params();

        if self.is_grounded() {
            let gh = GroundedHypothesis {
                rule:          self.rule,
                resolution:    self.resolution,
                params:        self.params,
                requirements:  self.requirements,
                blocked_rules: self.blocked_rules,
                pos:           self.pos,
            };
            return Either::Left(Some(gh).into_iter());
        }

        let generators = self.extract_generators();
        if generators.is_empty() {
            trace!(
                target: "rule_selection",
                "hypothesis has free params {:?}, no set generator found",
                self.free_params
            );
            return Either::Left(None.into_iter());
        }

        let combos = generators
            .into_iter()
            .map(|(param, elements)| {
                elements
                    .into_iter()
                    .map(move |e| (param.clone(), e))
                    .collect::<Vec<_>>()
            })
            .multi_cartesian_product()
            .filter_map(move |combo| {
                let mut inner = self.clone();
                inner.substitute_iter(combo);
                inner.bind_equality_params();

                if inner.is_grounded() {
                    Some(GroundedHypothesis {
                        rule:          inner.rule,
                        resolution:    inner.resolution,
                        params:        inner.params,
                        requirements:  inner.requirements,
                        blocked_rules: inner.blocked_rules,
                        pos:           inner.pos,
                    })
                } else {
                    trace!(
                        target: "rule_selection",
                        "hypothesis still has free params {:?} after generator",
                        inner.free_params
                    );
                    None
                }
            });
        Either::Right(combos)
    }

    /// Normalizes requirements and drops trivially-true ones.
    fn normalize_requirements(&mut self) {
        self.requirements = self
            .requirements
            .drain(..)
            .map(|r| r.normalize(NormLevel::Full))
            .filter(|r| r.term().truth() != Truth::True)
            .collect();
    }

    /// Finds and applies `param == term` requirements recursively.
    fn bind_equality_params(&mut self) {
        self.normalize_requirements();

        let mut candidates: Vec<(Param, TermBuf)> = Vec::new();
        self.requirements.retain(|r| {
            let Some((lhs, rhs)) = match_term!(r.term(), "=="(lhs, rhs)) else {
                return true;
            };
            if let Some(p) = lhs.data().param() {
                candidates.push((p.clone(), rhs.to_owned()));
                false
            } else if let Some(p) = rhs.data().param() {
                candidates.push((p.clone(), lhs.to_owned()));
                false
            } else {
                true
            }
        });

        while let Some(idx) = candidates
            .iter()
            .position(|(_, v)| !v.term().contains_params())
        {
            let (param, value) = candidates.swap_remove(idx);
            let subst: ParamSubstitution = [(param, value)].into_iter().collect();

            self.substitute(&subst);
            for (_, v) in &mut candidates {
                v.substitute(&subst);
            }
        }

        for (param, value) in candidates {
            self.requirements.push(
                TermBuf::symbol("==")
                    .arg(TermBuf::from(Atom::Param(param)))
                    .arg(value),
            );
        }

        self.normalize_requirements();
    }

    /// Extracts `param in set(...)` requirements as `(param, elements)`.
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

impl Substitute for Hypothesis {
    /// Applies `subst` to resolution + requirements and updates `free_params`
    /// / `params`.
    fn substitute(&mut self, subst: &ParamSubstitution) {
        self.resolution.substitute(subst);
        for r in &mut self.requirements {
            r.substitute(subst);
        }
        for (k, v) in &subst.params {
            self.free_params.remove(k);
            self.params.params.insert(k.clone(), v.clone());
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        rule::parse_rule,
        term::{ParamSubstitution, TermBuf, param, term_with_params, term_with_vars},
    };

    use super::Hypothesis;

    fn make_hypothesis(
        resolution: TermBuf,
        free_params: Vec<&str>,
        requirements: Vec<TermBuf>,
    ) -> Hypothesis {
        let rule = Arc::new(parse_rule(
            r#"rule { attr level(1); a + x == 0 => x == -a; a!=0; }"#,
        ));

        Hypothesis {
            rule,
            resolution,
            free_params: free_params.into_iter().map(param).collect(),
            params: ParamSubstitution::default(),
            requirements,
            blocked_rules: vec![],
            pos: Default::default(),
        }
    }

    fn res(g: &super::GroundedHypothesis) -> String {
        g.resolution.to_string()
    }

    #[test]
    fn ground_no_free_params() {
        let h = make_hypothesis(term_with_vars("x == 1"), vec![], vec![]);
        let grounded: Vec<_> = h.ground().collect();
        assert_eq!(grounded.len(), 1);
        assert_eq!(res(&grounded[0]), "x==1");
        assert!(grounded[0].requirements.is_empty());
    }

    #[test]
    fn ground_keeps_parents_guard() {
        // `parents` is inert without a position → guard stays Unknown, survives
        // grounding.
        let h = make_hypothesis(
            term_with_vars("answer(x == 1)"),
            vec![],
            vec![term_with_vars("!(answer in parents)")],
        );
        let grounded: Vec<_> = h.ground().collect();
        assert_eq!(grounded.len(), 1);
        assert_eq!(grounded[0].requirements.len(), 1);
        assert_eq!(
            grounded[0].requirements[0],
            term_with_vars("!(answer in parents)")
        );
    }

    #[test]
    fn ground_binds_equality_param() {
        let h = make_hypothesis(
            term_with_params("x == d"),
            vec!["d"],
            vec![term_with_params("-6 == d")],
        );
        let grounded: Vec<_> = h.ground().collect();
        assert_eq!(grounded.len(), 1);
        assert_eq!(res(&grounded[0]), "x==-6");
        assert!(grounded[0].requirements.is_empty());
    }

    #[test]
    fn ground_binds_equality_param_reversed() {
        let h = make_hypothesis(
            term_with_params("x == d"),
            vec!["d"],
            vec![term_with_params("d == -6")],
        );
        let grounded: Vec<_> = h.ground().collect();
        assert_eq!(grounded.len(), 1);
        assert_eq!(res(&grounded[0]), "x==-6");
    }

    #[test]
    fn ground_generator_only() {
        let h = make_hypothesis(
            term_with_params("x == u"),
            vec!["u"],
            vec![term_with_params("u in set(1, 2)")],
        );
        let grounded: Vec<_> = h.ground().collect();
        assert_eq!(grounded.len(), 2);

        let mut resolutions: Vec<String> = grounded.iter().map(res).collect();
        resolutions.sort();
        assert_eq!(resolutions, vec!["x==1", "x==2"]);
    }

    #[test]
    fn ground_equality_then_generator() {
        let h = make_hypothesis(
            term_with_params("x == u"),
            vec!["d", "u"],
            vec![
                term_with_params("-6 == d"),
                term_with_params("u in set(1, 2, 3, 6)"),
            ],
        );
        let grounded: Vec<_> = h.ground().collect();
        assert_eq!(grounded.len(), 4);
    }

    #[test]
    fn ground_equality_after_generator() {
        let resolution = TermBuf::symbol("||")
            .arg(
                TermBuf::symbol("==")
                    .arg(TermBuf::variable("x"))
                    .arg(TermBuf::param("u")),
            )
            .arg(
                TermBuf::symbol("==")
                    .arg(TermBuf::param("Q"))
                    .arg(TermBuf::number(0)),
            );
        let in_set = TermBuf::symbol("in")
            .arg(TermBuf::param("u"))
            .arg(TermBuf::symbol("set").arg(TermBuf::number(1)));
        let xplus5_eq_q = TermBuf::symbol("==")
            .arg(
                TermBuf::symbol("+")
                    .arg(TermBuf::variable("x"))
                    .arg(TermBuf::number(5)),
            )
            .arg(TermBuf::param("Q"));

        let h = make_hypothesis(resolution, vec!["u", "Q"], vec![in_set, xplus5_eq_q]);
        let grounded: Vec<_> = h.ground().collect();
        assert_eq!(grounded.len(), 1);
        assert_eq!(res(&grounded[0]), "||(x==1, x+5==0)");
        assert!(grounded[0].requirements.is_empty());
    }

    #[test]
    fn ground_filters_true_requirements() {
        let h = make_hypothesis(
            term_with_params("x == d"),
            vec!["d"],
            vec![term_with_params("-6 == d"), term_with_vars("1 != 0")],
        );
        let grounded: Vec<_> = h.ground().collect();
        assert_eq!(grounded.len(), 1);
        assert!(grounded[0].requirements.is_empty());
    }

    #[test]
    fn ground_keeps_non_trivial_requirements() {
        let h = make_hypothesis(
            term_with_params("x == d"),
            vec!["d"],
            vec![term_with_params("-6 == d"), term_with_vars("x != 0")],
        );
        let grounded: Vec<_> = h.ground().collect();
        assert_eq!(grounded.len(), 1);
        assert_eq!(grounded[0].requirements.len(), 1);
        assert_eq!(grounded[0].requirements[0].to_string(), "x!=0");
    }

    #[test]
    fn ground_no_generators_no_equality_returns_empty() {
        let h = make_hypothesis(
            term_with_params("x == d"),
            vec!["d"],
            vec![term_with_vars("x != 0")],
        );
        let grounded: Vec<_> = h.ground().collect();
        assert!(grounded.is_empty());
    }

    #[test]
    fn ground_iterative_equality_binding() {
        let h = make_hypothesis(
            term_with_params("x == d"),
            vec!["d"],
            vec![term_with_params("-6 == d"), term_with_params("d != 0")],
        );
        let grounded: Vec<_> = h.ground().collect();
        assert_eq!(grounded.len(), 1);
        assert!(grounded[0].requirements.is_empty());
    }
}

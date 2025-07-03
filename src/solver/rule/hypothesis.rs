use std::{fmt, rc::Rc};

use crate::term::{ParamsMapping, Term, TermProps};
use utils::VecDisplay;

use super::{ApplyRule, SharedRule};

#[derive(Debug)]
pub struct Hypothesis {
    pub requirements: Vec<Rc<Term>>,
    pub resolution:   TermProps,
    pub params:       ParamsMapping,
}

pub enum HypothesisIterator {
    Empty,
    Iter(std::vec::IntoIter<Hypothesis>),
}

impl Hypothesis {
    #[inline]
    pub fn rule(&self) -> Option<SharedRule> {
        self.resolution.inference.rule.clone()
    }

    #[inline]
    pub fn parent_idx(&self) -> Option<usize> {
        self.resolution.inference.parent
    }
}

impl HypothesisIterator {
    pub fn new(rule: SharedRule, term: TermProps, purpose: &TermProps) -> Self {
        let hypothesis = match rule.apply(&term, purpose) {
            Ok(x) => x,
            Err(e) => {
                trace!(target: "rule_selection", "rule {rule} not applied to term {term}: {e:?}" );
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
            VecDisplay(&self.requirements),
            self.resolution,
        )
    }
}

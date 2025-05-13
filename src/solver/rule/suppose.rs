use std::{fmt, rc::Rc};

use crate::term::{ParamsMapping, Term, TermProps};
use utils::VecDisplay;

use super::{ApplyRule, SharedRule};

#[derive(Debug)]
pub struct Suppose {
    pub requirements: Vec<Rc<Term>>,
    pub resolution:   TermProps,
    pub params:       ParamsMapping,
}

pub enum SupposesIterator {
    Empty,
    Iter(std::vec::IntoIter<Suppose>),
}

impl Suppose {
    #[inline]
    pub fn rule(&self) -> Option<SharedRule> {
        self.resolution.rule.clone()
    }

    #[inline]
    pub fn parent_idx(&self) -> Option<usize> {
        self.resolution.parent
    }
}

impl SupposesIterator {
    pub fn new(rule: SharedRule, term: TermProps, purpose: &TermProps) -> Self {
        let supposes = match rule.apply(&term, purpose) {
            Ok(x) => x,
            Err(e) => {
                trace!(target: "rule_selection", "rule {rule} not applied to term {term}: {e:?}" );
                return Self::empty();
            }
        };

        Self::Iter(supposes.into_iter())
    }

    pub fn empty() -> Self {
        Self::Empty
    }
}

impl Iterator for SupposesIterator {
    type Item = Suppose;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::Iter(i) => i.next(),
        }
    }
}

impl fmt::Display for Suppose {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{}] => {}",
            VecDisplay(&self.requirements),
            self.resolution,
        )
    }
}

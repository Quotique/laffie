use std::collections::HashSet;

use super::{rule::Level, RuleId};
use crate::term::Symbol;

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Default, Clone, Copy)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TermFlags: u32 {
        const REPLACED   = 0b0001;
        const SIMPLIFIED = 0b0010;
        const GOAL       = 0b0100;
    }
}

#[derive(Debug, Clone, Default)]
pub struct TermFilters {
    pub symbols:       HashSet<Symbol>,
    pub applied_rules: HashSet<RuleId>,
    pub blocked_rules: HashSet<RuleId>,
    pub level:         Level,
    flags:             TermFlags,
}

impl<I: IntoIterator<Item = Symbol>> From<I> for TermFilters {
    fn from(value: I) -> Self {
        Self {
            symbols: value.into_iter().collect(),
            ..Default::default()
        }
    }
}

impl TermFilters {
    pub fn is_replaced(&self) -> bool {
        self.flags & TermFlags::REPLACED == TermFlags::REPLACED
    }

    pub fn is_simplified(&self) -> bool {
        self.flags & TermFlags::SIMPLIFIED == TermFlags::SIMPLIFIED
    }

    pub fn is_goal(&self) -> bool {
        self.flags & TermFlags::GOAL == TermFlags::GOAL
    }

    pub fn mark_replaced(&mut self) {
        self.flags |= TermFlags::REPLACED;
    }

    pub fn mark_simplified(&mut self) {
        self.flags |= TermFlags::SIMPLIFIED;
    }

    pub fn mark_goal(&mut self) {
        self.flags |= TermFlags::GOAL;
    }
}

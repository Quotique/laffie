use std::collections::{HashMap, HashSet};

use super::{RuleId, rule::Level};
use crate::term::Symbol;

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Default, Clone, Copy)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TermFlags: u32 {
        const REPLACED   = 0b0001;
        const SIMPLIFIED = 0b0010;
        const GOAL       = 0b0100;
        const PREMISE    = 0b1000;
    }
}

/// The goals under which an exhausted rule stays exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExhaustionScope {
    AnyGoal,
    ThisGoal,
}

#[derive(Debug, Clone, Default)]
pub struct TermFilters {
    symbols:             Vec<Symbol>,
    /// Rules that yield nothing more on this term.
    pub exhausted_rules: HashMap<RuleId, ExhaustionScope>,
    pub blocked_rules:   HashSet<RuleId>,
    pub level:           Level,
    flags:               TermFlags,
}

impl<I: IntoIterator<Item = Symbol>> From<I> for TermFilters {
    fn from(value: I) -> Self {
        let mut symbols: Vec<Symbol> = value.into_iter().collect();
        symbols.sort_unstable();
        symbols.dedup();
        Self {
            symbols,
            ..Default::default()
        }
    }
}

impl TermFilters {
    /// Sorted and deduplicated.
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn contains_symbol(&self, symbol: &Symbol) -> bool {
        self.symbols.binary_search(symbol).is_ok()
    }

    pub fn is_replaced(&self) -> bool {
        self.flags & TermFlags::REPLACED == TermFlags::REPLACED
    }

    pub fn is_simplified(&self) -> bool {
        self.flags & TermFlags::SIMPLIFIED == TermFlags::SIMPLIFIED
    }

    pub fn is_goal(&self) -> bool {
        self.flags & TermFlags::GOAL == TermFlags::GOAL
    }

    pub fn is_premise(&self) -> bool {
        self.flags & TermFlags::PREMISE == TermFlags::PREMISE
    }

    pub fn mark_replaced(&mut self) {
        self.flags |= TermFlags::REPLACED;
    }

    pub fn mark_simplified(&mut self) {
        self.flags |= TermFlags::SIMPLIFIED;
    }

    pub fn mark_premise(&mut self) {
        self.flags |= TermFlags::PREMISE;
    }

    pub fn mark_goal(&mut self) {
        self.flags |= TermFlags::GOAL;
    }
}

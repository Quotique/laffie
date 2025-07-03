use std::{
    collections::HashSet,
    convert::From,
    fmt,
    hash::{Hash, Hasher},
    rc::Rc,
};

use bitflags::bitflags;

use super::{Symbol, Term};
use crate::{
    rule::{RuleAttr, RuleAttrValue, RuleBuilder, SharedRule},
    RuleId,
};

bitflags! {
    #[derive(Debug, Default, Clone, Copy)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TermFlags: u32 {
        const REPLACED   = 0b0001;
        const SIMPLIFIED = 0b0010;
        const PURPOSE    = 0b0100;
    }
}

#[derive(Debug, Clone, Default)]
pub struct TermInference {
    pub id:           usize,
    pub parent:       Option<usize>,
    pub rule:         Option<SharedRule>,
    pub requirements: Vec<Rc<Term>>,
}

#[derive(Debug, Clone, Default)]
pub struct TermFilters {
    pub func_symbols:  HashSet<Symbol>,
    pub applied_rules: HashSet<RuleId>,
    pub blocked_rules: HashSet<RuleId>,
    pub weight:        usize,
    flags:             TermFlags,
}

#[derive(Debug, Clone, Default)]
pub enum TermAsRule {
    #[default]
    Undefined,
    None,
    Rule(SharedRule),
}

#[derive(Debug, Clone)]
pub struct TermProps {
    pub term:      Rc<Term>,
    pub inference: TermInference,
    pub filters:   TermFilters,

    rule: TermAsRule,
}

impl From<Rc<Term>> for TermProps {
    fn from(value: Rc<Term>) -> Self {
        Self {
            inference: Default::default(),
            filters:   TermFilters {
                func_symbols: value.symbols(),
                ..Default::default()
            },
            term:      value,
            rule:      Default::default(),
        }
    }
}

impl TermFilters {
    pub fn mark_replaced(&mut self) {
        self.flags |= TermFlags::REPLACED;
    }

    pub fn is_replaced(&self) -> bool {
        self.flags & TermFlags::REPLACED == TermFlags::REPLACED
    }

    pub fn mark_simplified(&mut self) {
        self.flags |= TermFlags::SIMPLIFIED;
    }

    pub fn is_simplified(&self) -> bool {
        self.flags & TermFlags::SIMPLIFIED == TermFlags::SIMPLIFIED
    }

    pub fn mark_purpose(&mut self) {
        self.flags |= TermFlags::PURPOSE;
    }

    pub fn is_purpose(&self) -> bool {
        self.flags & TermFlags::PURPOSE == TermFlags::PURPOSE
    }
}

impl Hash for TermProps {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.term.hash(state);
    }
}

impl Eq for TermProps {}
impl PartialEq for TermProps {
    fn eq(&self, other: &Self) -> bool {
        self.term.as_ref() == other.term.as_ref()
    }
}

impl fmt::Display for TermProps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.term)
    }
}

impl TermAsRule {
    pub fn get_or_insert(&mut self, term: &Term, id: RuleId, level: u64) -> Option<SharedRule> {
        match self {
            TermAsRule::None => None,
            TermAsRule::Rule(r) => Some(r.clone()),
            TermAsRule::Undefined => {
                let result = Self::build_rule(term, id, level);
                *self = match &result {
                    Some(r) => TermAsRule::Rule(r.clone()),
                    None => TermAsRule::None,
                };
                result
            }
        }
    }

    fn build_rule(term: &Term, id: RuleId, level: u64) -> Option<SharedRule> {
        let builder = RuleBuilder::default()
            .with_id(id)
            .with_attribute(RuleAttr::Level, RuleAttrValue::UInt(level))
            .with_term(term.clone())
            .ok()?;
        let mut builded = builder
            .build()
            .map_err(|e| trace!("Error rule build: {} for  {}", e, term))
            .ok()?;
        let rule = builded
            .pop()
            .filter(|r| r.pattern_node().data().variable().is_some())?;
        trace!("New rule: {}", rule);
        Some(SharedRule::new(rule))
    }
}
impl TermProps {
    pub fn with_parent(mut self, id: usize) -> Self {
        if self.inference.parent.replace(id).is_some() {
            warn!("parent replacement");
        }
        self
    }

    pub fn without_parents(mut self) -> Self {
        self.inference.parent.take();
        self
    }

    pub fn with_rule(mut self, rule: SharedRule) -> Self {
        self.inference.rule = Some(rule);
        self
    }

    pub fn rule(&mut self, id: RuleId, level: u64) -> Option<SharedRule> {
        self.rule.get_or_insert(&self.term, id, level).inspect(|r| {
            self.filters.blocked_rules.insert(r.id);
        })
    }
}

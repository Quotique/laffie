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
        const NOT_RULE   = 0b1000;
    }
}

#[derive(Debug, Clone)]
pub struct TermProps {
    pub id:           usize,
    pub parent:       Option<usize>,
    pub rule:         Option<SharedRule>,
    pub requirements: Vec<Rc<Term>>,

    pub term:         Rc<Term>,
    pub func_symbols: HashSet<Symbol>,
    as_rule:          Option<SharedRule>,

    pub applied_rules: HashSet<RuleId>,
    pub blocked_rules: HashSet<RuleId>,
    pub weight:        usize,
    flags:             TermFlags,
}

impl From<Rc<Term>> for TermProps {
    fn from(value: Rc<Term>) -> Self {
        Self {
            id:           0,
            parent:       None,
            rule:         None,
            requirements: Default::default(),

            func_symbols: value.symbols(),
            term:         value,
            as_rule:      None,

            applied_rules: HashSet::new(),
            blocked_rules: HashSet::new(),
            weight:        0,
            flags:         Default::default(),
        }
    }
}

impl fmt::Display for TermProps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.term)
    }
}

impl Hash for TermProps {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.term.hash(state);
    }
}

impl PartialEq for TermProps {
    fn eq(&self, other: &Self) -> bool {
        self.term.as_ref() == other.term.as_ref()
    }
}

impl Eq for TermProps {}

impl TermProps {
    pub fn with_parent(mut self, id: usize) -> Self {
        if self.parent.replace(id).is_some() {
            warn!("parent replacement");
        }
        self
    }

    pub fn without_parents(mut self) -> Self {
        self.parent.take();
        self
    }

    pub fn with_rule(mut self, rule: SharedRule) -> Self {
        self.rule = Some(rule);
        self
    }

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

    pub fn rule(&mut self, id: RuleId, level: u64) -> Option<SharedRule> {
        if self.flags & TermFlags::NOT_RULE == TermFlags::NOT_RULE {
            return None;
        }

        if let Some(rule) = &self.as_rule {
            return Some(rule.clone());
        }

        if let Ok(builder) = RuleBuilder::default()
            .with_id(id)
            .with_attribute(RuleAttr::Level, RuleAttrValue::UInt(level))
            .with_term((*self.term).clone())
        {
            if let Ok(mut rule) = builder
                .build()
                .map_err(|e| trace!("Error rule build: {} for  {}", e, self.term))
            {
                if let Some(rule) = rule.pop() {
                    if rule.pattern_node().data().variable().is_some() {
                        trace!("New rule: {}", rule);
                        let rule = SharedRule::new(rule);
                        self.as_rule = Some(rule.clone());
                        self.blocked_rules.insert(id);
                        return Some(rule);
                    }
                }
            }
        }
        self.flags |= TermFlags::NOT_RULE;
        None
    }
}

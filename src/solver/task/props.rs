use std::{
    convert::From,
    fmt,
    hash::{Hash, Hasher},
    rc::Rc,
};

use crate::{
    rule::{RuleAttr, RuleAttrValue, RuleBuilder, RuleId, SharedRule, TermFilters},
    term::Term,
};

#[derive(Debug, Clone)]
pub enum Cause {
    Rule(SharedRule),
    Transform,
}

#[derive(Debug, Clone)]
pub struct TermInference {
    pub parent:       usize,
    pub rule:         Cause,
    pub requirements: Vec<Rc<Term>>,
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
    pub id:        usize,
    pub term:      Rc<Term>,
    pub inference: Option<TermInference>,
    pub filters:   TermFilters,

    rule: TermAsRule,
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cause::Rule(r) => write!(f, "{r}"),
            Cause::Transform => write!(f, "Transform"),
        }
    }
}

impl From<Rc<Term>> for TermProps {
    fn from(value: Rc<Term>) -> Self {
        Self {
            id:        Default::default(),
            inference: Default::default(),
            filters:   TermFilters::from(value.symbols()),
            term:      value,
            rule:      Default::default(),
        }
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
    pub fn rule(&mut self, id: RuleId, level: u64) -> Option<SharedRule> {
        self.rule.get_or_insert(&self.term, id, level).inspect(|r| {
            self.filters.blocked_rules.insert(r.id);
        })
    }
}

use std::{
    convert::From,
    fmt,
    hash::{Hash, Hasher},
};

use super::SharedSolution;
use crate::{
    rule::{RuleAttr, RuleAttrValue, RuleBuilder, RuleId, SharedRule, TermFilters},
    term::{SharedTerm, Term},
};

#[derive(Debug, Clone, Default)]
pub enum TermInference {
    Rule {
        parent:       usize,
        rule:         SharedRule,
        requirements: Vec<SharedSolution>,
    },
    Transform {
        parent:   usize,
        solution: SharedSolution,
    },
    #[default]
    Condition,
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
    pub term:      SharedTerm,
    pub inference: TermInference,
    pub filters:   TermFilters,

    rule: TermAsRule,
}

impl From<Term> for TermProps {
    fn from(value: Term) -> Self {
        Self::from(SharedTerm::new(value))
    }
}

impl From<SharedTerm> for TermProps {
    fn from(value: SharedTerm) -> Self {
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

impl TermInference {
    pub fn requirements(&self) -> Option<&Vec<SharedSolution>> {
        match self {
            TermInference::Rule { requirements, .. } => Some(requirements),
            _ => None,
        }
    }

    pub fn is_proven(&self) -> bool {
        match self {
            TermInference::Rule { requirements, .. } => {
                requirements.iter().all(|x| x.answer().is_some())
            }
            _ => true,
        }
    }

    pub fn rule(&self) -> Option<SharedRule> {
        match self {
            TermInference::Rule { rule, .. } => Some(rule.clone()),
            _ => None,
        }
    }

    pub fn parent_id(&self) -> Option<usize> {
        match self {
            TermInference::Rule { parent, .. } => Some(*parent),
            TermInference::Transform { parent, .. } => Some(*parent),
            _ => None,
        }
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
            .map_err(|e| trace!("Error rule build: {e} for  {term}"))
            .ok()?;
        let rule = builded
            .pop()
            .filter(|r| r.pattern_node().data().variable().is_some())?;
        trace!("New rule: {rule}");
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

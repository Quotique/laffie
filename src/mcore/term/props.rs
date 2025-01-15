use std::{
    collections::HashSet,
    convert::From,
    fmt,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::Arc,
};

use super::{FuncSymbol, Term};
use crate::{
    rule::{RuleAttr, RuleAttrValue, RuleBuilder, SharedRule},
    RuleId,
};

#[derive(Debug, Clone)]
pub struct TermProps {
    pub id:           usize,
    pub parent:       Option<usize>,
    pub rule:         Option<SharedRule>,
    pub requirements: Vec<Rc<Term>>,

    pub term:         Rc<Term>,
    pub func_symbols: HashSet<Arc<FuncSymbol>>,
    as_rule:          Option<SharedRule>,

    pub applied_rules: HashSet<RuleId>,
    pub blocked_rules: HashSet<RuleId>,
    pub weight:        usize,
    pub replaced:      bool,
    pub simplified:    bool,
    not_rule:          bool,
    pub is_purpose:    bool,
}

impl From<Rc<Term>> for TermProps {
    fn from(value: Rc<Term>) -> Self {
        Self {
            id:           0,
            parent:       None,
            rule:         None,
            requirements: Default::default(),

            func_symbols: value.func_symbols(),
            term:         value,
            as_rule:      None,

            applied_rules: HashSet::new(),
            blocked_rules: HashSet::new(),
            weight:        0,
            replaced:      false,
            simplified:    false,
            not_rule:      false,
            is_purpose:    false,
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

    pub fn rule(&mut self, id: RuleId, level: u64) -> Option<SharedRule> {
        if self.not_rule {
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
                        let rule = Arc::new(rule);
                        self.as_rule = Some(rule.clone());
                        self.blocked_rules.insert(id);
                        return Some(rule);
                    }
                }
            }
        }
        self.not_rule = true;
        None
    }
}

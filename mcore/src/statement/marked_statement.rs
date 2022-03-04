use super::statement::Statement;
use crate::rule::{Rule, RuleAttr, RuleAttrValue, RuleBuilder};
use std::{
    collections::HashSet,
    convert::From,
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

use parking_lot::RwLock;

#[derive(Debug, Clone)]
pub struct MarkedStatement {
    pub id:      usize,
    pub parents: Vec<usize>,
    pub rule:    Option<Arc<RwLock<Rule>>>,

    pub statement: Arc<Statement>,
    pub symbols:   HashSet<u64>,
    as_rule:       Option<Arc<RwLock<Rule>>>,

    pub applied_rules: HashSet<usize>,
    pub blocked_rules: HashSet<usize>,
    pub weight:        usize,
    pub replaced:      bool,
    pub simplified:    bool,
    not_rule:          bool,
}

impl From<Arc<Statement>> for MarkedStatement {
    fn from(value: Arc<Statement>) -> Self {
        Self {
            id:      0,
            parents: vec![],
            rule:    None,

            symbols:   value.symbols(),
            statement: value,
            as_rule:   None,

            applied_rules: HashSet::new(),
            blocked_rules: HashSet::new(),
            weight:        0,
            replaced:      false,
            simplified:    false,
            not_rule:      false,
        }
    }
}

impl fmt::Display for MarkedStatement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.statement)
    }
}

impl Hash for MarkedStatement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.statement.hash(state);
    }
}

impl PartialEq for MarkedStatement {
    fn eq(&self, other: &Self) -> bool {
        self.statement.as_ref() == other.statement.as_ref()
    }
}

impl Eq for MarkedStatement {}

impl MarkedStatement {
    pub fn with_parent(mut self, id: usize) -> Self {
        self.parents.push(id);
        self
    }

    pub fn rule(&mut self, id: usize, level: u64) -> Option<Arc<RwLock<Rule>>> {
        if self.not_rule {
            return None;
        }

        if let Some(rule) = &self.as_rule {
            return Some(rule.clone());
        }

        if let Ok(builder) = RuleBuilder::default()
            .with_id(id)
            .with_attribute(RuleAttr::Level, RuleAttrValue::UInt(level))
            .with_statement((*self.statement).clone())
        {
            if let Ok(mut rule) = builder
                .build()
                .map_err(|e| trace!("Error rule build: {} for  {}", e, self.statement))
            {
                if let Some(rule) = rule.pop() {
                    if rule.pattern_node().data().variable().is_some() {
                        trace!("New rule: {}", rule);
                        let rule = Arc::new(RwLock::new(rule));
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

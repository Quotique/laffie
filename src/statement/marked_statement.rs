use super::statement::Statement;
use crate::core::rule::Rule;
use std::{
    collections::HashSet,
    convert::From,
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
pub struct MarkedStatement {
    pub parents: Vec<Arc<MarkedStatement>>,
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
    fn from(statement: Arc<Statement>) -> Self {
        Self {
            parents: vec![],
            rule:    None,

            symbols:   statement.symbols(),
            statement: statement,
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
        *self.statement == *self.statement
    }
}

impl Eq for MarkedStatement {}

impl MarkedStatement {
    pub fn normalize(self) -> Self {
        let mut copy = self.clone();
        copy.statement = Arc::new(self.statement.normalize());
        copy
    }

    pub fn rule(&mut self) -> Option<Arc<RwLock<Rule>>> {
        None
    }
}

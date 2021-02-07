use super::rule::{Rule, RuleAttr, RuleAttrValue};
use crate::{
    core::{symbols::symbol_by_name, term::Term},
    statement::Statement,
};
use std::{collections::HashMap, convert::From, fmt};

#[derive(Clone, Debug)]
pub enum RuleBuilderError {
    BadStatementRoot,
    WrongArgsCount,
    OnlyOneStatementIsAllowed,
    MissingLevelAttribute,
}

pub struct RuleBuilder {
    rule_id:      usize,
    statement:    Option<Statement>,
    requirements: Vec<Statement>,
    attributes:   Vec<(RuleAttr, RuleAttrValue)>,
    symbol_id:    u64,
}

impl From<Statement> for RuleBuilder {
    fn from(source: Statement) -> Self {
        Self::new().with_statement(source).unwrap()
    }
}

impl fmt::Display for RuleBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BadStatementRoot => write!(f, "Bad statement root"),
            Self::WrongArgsCount => write!(f, "Wrong args count"),
            Self::OnlyOneStatementIsAllowed => write!(f, "Only one statementis allowed"),
            Self::MissingLevelAttribute => write!(f, "Missing level attribute"),
        }
    }
}

impl RuleBuilder {
    pub fn new() -> Self {
        RuleBuilder {
            rule_id:      0,
            statement:    None,
            requirements: vec![],
            attributes:   vec![],
            symbol_id:    symbol_by_name(&"AnySymbol".into())
                .expect("System symbol AnySymbol is not found")
                .id,
        }
    }

    pub fn with_id(mut self, id: usize) -> Self {
        self.rule_id = id;
        self
    }

    pub fn with_symbol_id(mut self, symbol_id: u64) -> Self {
        self.symbol_id = symbol_id;
        self
    }

    pub fn with_statement(mut self, statement: Statement) -> Result<Self, RuleBuilderError> {
        if let Some(_) = self.statement.replace(statement) {
            return Err(RuleBuilderError::OnlyOneStatementIsAllowed);
        }
        Ok(self)
    }

    pub fn with_require(mut self, requirement: Statement) -> Self {
        self.requirements.push(requirement);
        self
    }

    pub fn with_attribute(mut self, attr: RuleAttr, value: RuleAttrValue) -> Self {
        self.attributes.push((attr, value));
        self
    }

    pub fn with_requires<I: IntoIterator<Item = Statement>>(mut self, reqs: I) -> Self {
        self.requirements.extend(reqs);
        self
    }

    pub fn with_attributes<I>(mut self, attrs: I) -> Self
    where
        I: IntoIterator<Item = (RuleAttr, RuleAttrValue)>,
    {
        self.attributes.extend(attrs);
        self
    }

    pub fn build(mut self) -> Result<Rule, RuleBuilderError> {
        let (pattern, replace) = self.split_statement()?;
        let attrs: HashMap<RuleAttr, RuleAttrValue> = self.attributes.into_iter().collect();

        let level = if let Some(RuleAttrValue::UInt(level)) = attrs.get(&RuleAttr::Level) {
            level
        } else {
            return Err(RuleBuilderError::MissingLevelAttribute);
        };

        Ok(Rule {
            id:        self.rule_id,
            level:     *level as usize,
            symbol_id: self.symbol_id,

            attrs:           attrs,
            pattern_symbols: pattern.symbols(),

            pattern: pattern,
            replace: replace,

            requirements: self.requirements,
        })
    }

    fn split_statement(&mut self) -> Result<(Statement, Statement), RuleBuilderError> {
        let (root, mut childs) = self
            .statement
            .take()
            .ok_or(RuleBuilderError::BadStatementRoot)?
            .destruct();

        if childs.degree() != 2 {
            return Err(RuleBuilderError::WrongArgsCount);
        }

        if root.data == Term::with_symbol_name("=>").unwrap() {
            return Ok((
                Statement::from(childs.pop_front().unwrap()),
                Statement::from(childs.pop_back().unwrap()),
            ));
        } else if root.data == Term::with_symbol_name("<=>").unwrap() {
            self.attributes
                .push((RuleAttr::Equivalence, RuleAttrValue::None));

            return Ok((
                Statement::from(childs.pop_front().unwrap()),
                Statement::from(childs.pop_back().unwrap()),
            ));
        } else if root.data == Term::with_symbol_name("==").unwrap() {
            self.attributes
                .push((RuleAttr::Equivalence, RuleAttrValue::None));
            self.attributes
                .push((RuleAttr::Subtree, RuleAttrValue::None));

            return Ok((
                Statement::from(childs.pop_front().unwrap()),
                Statement::from(childs.pop_back().unwrap()),
            ));
        }

        Err(RuleBuilderError::BadStatementRoot)
    }
}

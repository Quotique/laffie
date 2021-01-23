use super::rule::{Rule, RuleAttr, RuleAttrValue};
use crate::{core::symbols::symbol_by_name, statement::Statement};
use std::{collections::HashMap, convert::From};

#[derive(Clone, Debug)]
pub enum RuleBuilderError {
    BadStatementRoot,
    OnlyOneStatementIsAllowed,
    MissingLevelAttribute,
}

pub struct RuleBuilder {
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

impl RuleBuilder {
    pub fn new() -> Self {
        RuleBuilder {
            statement:    None,
            requirements: vec![],
            attributes:   vec![],
            symbol_id:    symbol_by_name(&"AnySymbol".into())
                .expect("System symbol AnySymbol is not found")
                .id,
        }
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
            id:        0,
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
        Err(RuleBuilderError::BadStatementRoot)
        // TODO: parse statement with params match

        // if self.statement.root.data.is_symbol_name(&"==".into()) {
        //     if self.root.first().unwrap().data.is_variable() {
        //         if !Self::contains(&self.root.first().unwrap().data,
        // &self.root.last().unwrap())         {
        //             let pattern = self.root.first().unwrap().to_owned();
        //             let pattern_symbols = symbols(&pattern);
        //             return Some(Rule {
        //                 id:              0,
        //                 level:           0,
        //                 attrs:           [(RuleAttr::Subtree,
        // RuleAttrValue::None)]                     .iter()
        //                     .cloned()
        //                     .collect(),
        //                 pattern:         pattern,
        //                 replace:
        // self.root.last().unwrap().to_owned(),
        // requirements:    vec![],                 pattern_symbols:
        // pattern_symbols,             });
        //         }
        //     }
        // } else if self.root.data.is_symbol_name(&"=>".into()) {
        //     let pattern = self.root.first().unwrap().to_owned();
        //     let pattern_symbols = symbols(&pattern);
        //     return Some(Rule {
        //         id:              0,
        //         level:           0,
        //         attrs:           HashMap::new(),
        //         pattern:         pattern,
        //         replace:         self.root.last().unwrap().to_owned(),
        //         requirements:    vec![],
        //         pattern_symbols: pattern_symbols,
        //     });
        // }
        //
        // return None;
    }
}

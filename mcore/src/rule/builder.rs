use std::{collections::HashMap, convert::From, fmt};

use multimap::MultiMap;

use crate::{
    predefine::symbol_by_name,
    statement::{NodeMapping, NodePosition, Statement},
    NormalizationLevel, RuleId, SymbolId,
};

use super::rule::{Rule, RuleAttr, RuleAttrValue};

#[derive(Clone, Debug)]
pub enum RuleBuilderError {
    BadStatementRoot,
    WrongArgsCount,
    OnlyOneStatementIsAllowed,
    MissingLevelAttribute,
}

pub struct RuleBuilder {
    rule_id:      RuleId,
    statement:    Option<Statement>,
    requirements: Vec<Statement>,
    attributes:   Vec<(RuleAttr, RuleAttrValue)>,
    symbol_id:    SymbolId,

    replaces: Vec<(RuleAttr, Statement)>,
}

impl From<Statement> for RuleBuilder {
    fn from(source: Statement) -> Self {
        Self::default().with_statement(source).unwrap()
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

impl Default for RuleBuilder {
    fn default() -> Self {
        RuleBuilder {
            rule_id:      RuleId::default(),
            statement:    None,
            requirements: Default::default(),
            attributes:   Default::default(),
            symbol_id:    symbol_by_name("AnySymbol")
                .expect("System symbol AnySymbol is not found")
                .id,
            replaces:     Default::default(),
        }
    }
}

impl RuleBuilder {
    pub fn with_id(mut self, id: RuleId) -> Self {
        self.rule_id = id;
        self
    }

    pub fn with_symbol_id(mut self, symbol_id: SymbolId) -> Self {
        self.symbol_id = symbol_id;
        self
    }

    pub fn with_statement(mut self, statement: Statement) -> Result<Self, RuleBuilderError> {
        if self.statement.replace(statement).is_some() {
            return Err(RuleBuilderError::OnlyOneStatementIsAllowed);
        }
        Ok(self)
    }

    pub fn with_require(mut self, requirement: Statement) -> Self {
        self.requirements.push(requirement);
        self
    }

    pub fn with_attribute(mut self, attr: RuleAttr, value: RuleAttrValue) -> Self {
        match (&attr, &value) {
            (RuleAttr::Zero, RuleAttrValue::Target(s)) |
            (RuleAttr::One, RuleAttrValue::Target(s)) => self.replaces.push((attr, s.clone())),
            _ => self.attributes.push((attr, value)),
        }
        self
    }

    pub fn build(mut self) -> Result<Vec<Rule>, RuleBuilderError> {
        let statement = self
            .statement
            .take()
            .ok_or(RuleBuilderError::BadStatementRoot)?;

        let root_sym = statement
            .root()
            .data()
            .symbol()
            .ok_or(RuleBuilderError::BadStatementRoot)?;

        match root_sym.name.as_str() {
            "=>" => {}
            "<=>" | "==" => {
                self.attributes
                    .push((RuleAttr::Equivalence, RuleAttrValue::None));
            }
            _ => return Err(RuleBuilderError::BadStatementRoot),
        }

        if statement.root().degree() != 2 {
            return Err(RuleBuilderError::WrongArgsCount);
        }

        let mut attrs: MultiMap<RuleAttr, RuleAttrValue> = self.attributes.into_iter().collect();
        attrs.insert(RuleAttr::Subtree, RuleAttrValue::None);

        let level = if let Some(RuleAttrValue::UInt(level)) = attrs.get(&RuleAttr::Level) {
            level
        } else {
            return Err(RuleBuilderError::MissingLevelAttribute);
        };
        let mut result = vec![];
        for set in 0..(2_u64).pow(self.replaces.len() as u32) {
            let mut statement = statement.clone();
            let mut reqs = self.requirements.clone();

            for i in 0..self.replaces.len() {
                let elem = 0b1 << i;
                if set & elem == elem {
                    match self.replaces.get(i) {
                        Some((RuleAttr::One, src)) => {
                            statement.replace(src, &Statement::one());
                            for i in reqs.iter_mut() {
                                i.replace(src, &Statement::one());
                            }
                        }
                        Some((RuleAttr::Zero, src)) => {
                            statement.replace(src, &Statement::zero());
                            for i in reqs.iter_mut() {
                                i.replace(src, &Statement::zero());
                            }
                        }
                        _ => {}
                    }
                }
            }
            let binds: HashMap<_, _> = statement
                .binds
                .iter()
                .map(|(param, pos)| (param.clone(), statement[pos].deep_clone()))
                .collect();

            // TODO: normalization level
            statement = statement.normalize(NormalizationLevel::max());

            result.push(Rule {
                id: self.rule_id,
                level: *level as usize,
                symbol_id: self.symbol_id,
                attrs: attrs.clone(),
                block: Default::default(),
                pattern_symbols: statement.root().front().unwrap().symbols(),
                statement,
                pattern: NodePosition::default().child(0),
                replace: NodePosition::default().child(1),
                binds: binds.into(),
                requirements: reqs,
            });
        }
        result.retain(|x| !x.is_tautology());
        Ok(result)
    }
}

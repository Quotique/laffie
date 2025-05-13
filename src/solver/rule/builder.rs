use std::{collections::HashMap, convert::From, fmt, sync::Arc};

use multimap::MultiMap;

use crate::{
    symbol::FuncSymbol,
    term::{NodeMapping, NodePosition, Term},
    NormalizationLevel, RuleId,
};

use super::{
    rule::Rule,
    rule_attribute::{RuleAttr, RuleAttrValue},
};

#[derive(Clone, Debug)]
pub enum RuleBuilderError {
    BadTermRoot,
    WrongArgsCount,
    OnlyOneTermIsAllowed,
    MissingLevelAttribute,
}

pub struct RuleBuilder {
    rule_id:      RuleId,
    term:         Option<Term>,
    requirements: Vec<Term>,
    attributes:   Vec<(RuleAttr, RuleAttrValue)>,
    func_symbol:  Arc<FuncSymbol>,

    replaces: Vec<(RuleAttr, Term)>,
}

impl From<Term> for RuleBuilder {
    fn from(source: Term) -> Self {
        Self::default().with_term(source).unwrap()
    }
}

impl fmt::Display for RuleBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BadTermRoot => write!(f, "Bad term root"),
            Self::WrongArgsCount => write!(f, "Wrong args count"),
            Self::OnlyOneTermIsAllowed => write!(f, "Only one termis allowed"),
            Self::MissingLevelAttribute => write!(f, "Missing level attribute"),
        }
    }
}

impl Default for RuleBuilder {
    fn default() -> Self {
        RuleBuilder {
            rule_id:      RuleId::default(),
            term:         None,
            requirements: Default::default(),
            attributes:   Default::default(),
            func_symbol:  FuncSymbol::by_name("AnySymbol")
                .expect("System symbol AnySymbol is not found"),
            replaces:     Default::default(),
        }
    }
}

impl RuleBuilder {
    pub fn with_id(mut self, id: RuleId) -> Self {
        self.rule_id = id;
        self
    }

    pub fn with_func_symbol(mut self, func_symbol: Arc<FuncSymbol>) -> Self {
        self.func_symbol = func_symbol;
        self
    }

    pub fn with_term(mut self, term: Term) -> Result<Self, RuleBuilderError> {
        if self.term.replace(term).is_some() {
            return Err(RuleBuilderError::OnlyOneTermIsAllowed);
        }
        Ok(self)
    }

    pub fn with_require(mut self, requirement: Term) -> Self {
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
        let term = self.term.take().ok_or(RuleBuilderError::BadTermRoot)?;

        let root_sym = term
            .root()
            .data()
            .func_symbol()
            .ok_or(RuleBuilderError::BadTermRoot)?;

        match root_sym.name.as_str() {
            "=>" => {}
            "<=>" | "==" => {
                self.attributes
                    .push((RuleAttr::Equivalence, RuleAttrValue::None));
            }
            _ => return Err(RuleBuilderError::BadTermRoot),
        }

        if term.root().degree() != 2 {
            return Err(RuleBuilderError::WrongArgsCount);
        }

        let attrs: MultiMap<RuleAttr, RuleAttrValue> = self.attributes.into_iter().collect();

        let level = if let Some(RuleAttrValue::UInt(level)) = attrs.get(&RuleAttr::Level) {
            level
        } else {
            return Err(RuleBuilderError::MissingLevelAttribute);
        };
        let mut result = vec![];
        for set in 0..(2_u64).pow(self.replaces.len() as u32) {
            let mut term = term.clone();
            let mut reqs = self.requirements.clone();

            for i in 0..self.replaces.len() {
                let elem = 0b1 << i;
                if set & elem == elem {
                    match self.replaces.get(i) {
                        Some((RuleAttr::One, src)) => {
                            term.replace(src, &Term::one());
                            for i in reqs.iter_mut() {
                                i.replace(src, &Term::one());
                            }
                        }
                        Some((RuleAttr::Zero, src)) => {
                            term.replace(src, &Term::zero());
                            for i in reqs.iter_mut() {
                                i.replace(src, &Term::zero());
                            }
                        }
                        _ => {}
                    }
                }
            }
            let binds: HashMap<_, _> = term
                .binds
                .iter()
                .map(|(param, pos)| (param.clone(), term[pos].deep_clone()))
                .collect();

            // TODO: normalization level
            term = term.normalize(NormalizationLevel::from(1));

            result.push(Rule {
                id: self.rule_id,
                level: *level as usize,
                func_symbol: self.func_symbol.clone(),
                attrs: attrs.clone(),
                block: Default::default(),
                pattern_symbols: term.root().front().unwrap().symbols(),
                term,
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

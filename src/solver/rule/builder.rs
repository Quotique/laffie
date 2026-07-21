use std::{convert::From, fmt};

use multimap::MultiMap;

use crate::{
    NormLevel,
    term::{Symbol, Term, TermBuf, sym},
};

use super::{Rule, RuleAttr, RuleAttrValue, RuleId};

#[derive(Clone, Debug)]
pub enum RuleBuilderError {
    BadTermRoot,
    WrongArgsCount,
    OnlyOneTermIsAllowed,
    MissingLevelAttribute,
}

pub struct RuleBuilder {
    rule_id:      RuleId,
    term:         Option<TermBuf>,
    requirements: Vec<TermBuf>,
    attributes:   Vec<(RuleAttr, RuleAttrValue)>,
    symbol:       Symbol,

    replaces: Vec<(RuleAttr, TermBuf)>,
}

impl From<TermBuf> for RuleBuilder {
    fn from(source: TermBuf) -> Self {
        Self::default().with_term(source).unwrap()
    }
}

impl fmt::Display for RuleBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BadTermRoot => write!(f, "Bad term root"),
            Self::WrongArgsCount => write!(f, "Wrong args count"),
            Self::OnlyOneTermIsAllowed => write!(f, "Only one term is allowed"),
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
            symbol:       sym("AnySymbol"),
            replaces:     Default::default(),
        }
    }
}

impl RuleBuilder {
    pub fn with_id(mut self, id: RuleId) -> Self {
        self.rule_id = id;
        self
    }

    pub fn with_func_symbol(mut self, func_symbol: Symbol) -> Self {
        self.symbol = func_symbol;
        self
    }

    pub fn with_term(mut self, term: TermBuf) -> Result<Self, RuleBuilderError> {
        if self.term.replace(term).is_some() {
            return Err(RuleBuilderError::OnlyOneTermIsAllowed);
        }
        Ok(self)
    }

    pub fn with_require(mut self, requirement: TermBuf) -> Self {
        self.requirements.push(requirement);
        self
    }

    pub fn with_attribute(mut self, attr: RuleAttr, value: RuleAttrValue) -> Self {
        match (&attr, &value) {
            (RuleAttr::Zero, RuleAttrValue::Goal(s)) | (RuleAttr::One, RuleAttrValue::Goal(s)) => {
                self.replaces.push((attr, s.clone()))
            }
            _ => self.attributes.push((attr, value)),
        }
        self
    }

    pub fn build(mut self) -> Result<Vec<Rule>, RuleBuilderError> {
        let term = self.term.take().ok_or(RuleBuilderError::BadTermRoot)?;

        let root_sym = term
            .term()
            .data()
            .symbol()
            .ok_or(RuleBuilderError::BadTermRoot)?;

        match root_sym.as_str() {
            "=>" => {}
            "<=>" | "==" => {
                self.attributes
                    .push((RuleAttr::Equivalence, RuleAttrValue::None));
            }
            _ => return Err(RuleBuilderError::BadTermRoot),
        }

        if term.term().degree() != 2 {
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
                            term.replace(src, &TermBuf::one());
                            for i in reqs.iter_mut() {
                                i.replace(src, &TermBuf::one());
                            }
                        }
                        Some((RuleAttr::Zero, src)) => {
                            term.replace(src, &TermBuf::zero());
                            for i in reqs.iter_mut() {
                                i.replace(src, &TermBuf::zero());
                            }
                        }
                        _ => {}
                    }
                }
            }
            // TODO: normalization level
            term = term.normalize(NormLevel::Units);

            result.push(Rule {
                id: self.rule_id,
                level: (*level).into(),
                symbol: self.symbol.clone(),
                attrs: attrs.clone(),
                block: Default::default(),
                pattern_symbols: term.term().first_arg().unwrap().symbols(),
                pattern: term.term().first_arg().unwrap().id(),
                replace: term.term().last_arg().unwrap().id(),
                term,
                requirements: reqs,
            });
        }
        result.retain(|x| !x.is_tautology());
        Ok(result)
    }
}

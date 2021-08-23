use std::{fmt, hash::Hash};

pub use bigdecimal::{BigDecimal as Decimal, Signed};
use derive_more::{Display, From, FromStr, Into};
pub use smartstring::alias::String as CompactString;
use trees::{Node, Tree};

use super::symbols::{symbol_by_id, symbol_by_name, Symbol};

pub type StatementTree = Tree<Term>;

#[derive(Clone, Debug, Display, PartialEq, Eq, Hash, From, FromStr, Into, Ord, PartialOrd)]
pub struct Param(CompactString);
#[derive(Clone, Debug, Display, PartialEq, Eq, Hash, From, FromStr, Into, Ord, PartialOrd)]
pub struct Variable(CompactString);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    Symbol(u64),
    Param(Param),
    Variable(Variable),
    Number(Decimal),
}

impl Term {
    pub fn with_symbol_name(name: &str) -> Option<Self> {
        symbol_by_name(name).map(|s| Self::Symbol(s.id))
    }

    pub fn symbol(&self) -> Option<Symbol> {
        if let Term::Symbol(id) = self {
            return symbol_by_id(*id);
        }
        None
    }

    pub fn symbol_id(&self) -> Option<u64> {
        if let Term::Symbol(id) = self {
            return Some(*id);
        }
        None
    }

    pub fn variable(&self) -> Option<&Variable> {
        if let Term::Variable(v) = &self {
            return Some(v);
        }
        None
    }

    pub fn param(&self) -> Option<&Param> {
        if let Term::Param(p) = &self {
            return Some(p);
        }
        None
    }

    #[allow(dead_code)]
    pub fn is_symbol_name(&self, name: &str) -> bool {
        if let Some(s) = symbol_by_name(name) {
            return self.symbol_id() == Some(s.id);
        }
        false
    }

    pub fn is_number_value(&self, value: &Decimal) -> bool {
        if let Term::Number(num) = &self {
            return num == value;
        }
        false
    }
}

pub fn display_string(node: &Node<Term>) -> String {
    match node.data() {
        Term::Symbol(id) => {
            let symbol = symbol_by_id(*id).unwrap();
            match symbol.display_weight() {
                Some(weight) => {
                    if node.degree() < 2 {
                        return format!(
                            "{}{}",
                            symbol,
                            node.iter()
                                .map(|x| display_string(x))
                                .collect::<Vec::<String>>()
                                .join(", ")
                        );
                    }
                    node.iter()
                        .map(|x| {
                            if let Term::Symbol(id) = x.data() {
                                let symbol = symbol_by_id(*id).unwrap();
                                if let Some(other_weight) = symbol.display_weight() {
                                    if weight <= other_weight {
                                        return format!("({})", display_string(x));
                                    }
                                }
                            }
                            display_string(x)
                        })
                        .collect::<Vec<String>>()
                        .join(symbol.to_string().as_str())
                }
                None => {
                    // Prefix notation is default
                    if node.degree() > 0 {
                        format!(
                            "{}({})",
                            symbol,
                            node.iter()
                                .map(|x| display_string(x))
                                .collect::<Vec::<String>>()
                                .join(", ")
                        )
                    } else {
                        format!("{}", symbol,)
                    }
                }
            }
        }
        _ => node.data().to_string(),
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Term::Symbol(id) => {
                let s = symbol_by_id(*id).unwrap();
                write!(f, "{}", s.name)
            }
            Term::Param(id) => write!(f, "{}", id),
            Term::Number(value) => {
                if value.is_negative() {
                    write!(f, "({})", value)
                } else {
                    write!(f, "{}", value)
                }
            }
            Term::Variable(id) => write!(f, "{}", id),
        }
    }
}

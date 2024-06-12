use std::{fmt, hash::Hash, sync::Arc};

use derive_more::{AsRef, Display, From, FromStr, Into};

use super::func_symbol::FuncSymbol;
use crate::{predefine::symbol_by_name, CompactString, Decimal, Signed};

#[derive(Clone, Debug, Display)]
#[derive(PartialEq, Eq, Hash, AsRef, From, FromStr, Into, Ord, PartialOrd)]
pub struct Param(CompactString);

#[derive(Clone, Debug, Display)]
#[derive(PartialEq, Eq, Hash, AsRef, From, FromStr, Into, Ord, PartialOrd)]
pub struct Variable(CompactString);

#[derive(Clone, Copy, Debug, Display)]
#[derive(PartialEq, Eq, Hash, From, FromStr, Into, Ord, PartialOrd)]
pub struct Placeholder(u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Symbol {
    FuncSymbol(Arc<FuncSymbol>),
    Param(Param),
    Variable(Variable),
    Number(Decimal),
    Placeholder(Placeholder),
}

impl Symbol {
    #[inline]
    pub fn with_func_symbol_opt(name: &str) -> Option<Self> {
        symbol_by_name(name).map(Self::FuncSymbol)
    }

    #[inline]
    pub fn with_func_symbol(name: &str) -> Self {
        Self::with_func_symbol_opt(name).unwrap()
    }

    #[inline]
    pub fn with_number(number: impl Into<Decimal>) -> Self {
        Self::Number(number.into())
    }

    #[inline]
    pub fn func_symbol(&self) -> Option<Arc<FuncSymbol>> {
        if let Symbol::FuncSymbol(s) = self {
            return Some(s.clone());
        }
        None
    }

    #[inline]
    pub fn variable(&self) -> Option<&Variable> {
        if let Symbol::Variable(v) = &self {
            return Some(v);
        }
        None
    }

    #[inline]
    pub fn param(&self) -> Option<&Param> {
        if let Symbol::Param(p) = &self {
            return Some(p);
        }
        None
    }

    #[inline]
    pub fn number(&self) -> Option<&Decimal> {
        if let Symbol::Number(d) = &self {
            return Some(d);
        }
        None
    }

    #[inline]
    pub fn placeholder(&self) -> Option<&Placeholder> {
        if let Symbol::Placeholder(p) = &self {
            return Some(p);
        }
        None
    }

    #[inline]
    pub fn is_symbol_name(&self, name: &str) -> bool {
        if let Symbol::FuncSymbol(s) = self {
            return s.name == name;
        }

        false
    }

    #[inline]
    pub fn is_number_value(&self, value: &Decimal) -> bool {
        if let Symbol::Number(num) = &self {
            return num == value;
        }
        false
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Symbol::FuncSymbol(s) => {
                write!(f, "{}", s.name)
            }
            Symbol::Param(id) => write!(f, "{id}"),
            Symbol::Number(value) => {
                if value.is_negative() {
                    write!(f, "({value})")
                } else {
                    write!(f, "{value}")
                }
            }
            Symbol::Variable(id) => write!(f, "{id}"),
            Symbol::Placeholder(_) => write!(f, ".."),
        }
    }
}

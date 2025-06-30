use std::{cmp::Ordering, fmt, hash::Hash};

use derive_more::{AsRef, Display, From, FromStr, Into};

use super::symbol::Symbol;
use crate::{CompactString, Decimal, Signed};

#[derive(Clone, Debug, Display)]
#[derive(PartialEq, Eq, Hash, AsRef, From, FromStr, Into, Ord, PartialOrd)]
pub struct Param(CompactString);

#[derive(Clone, Debug, Display)]
#[derive(PartialEq, Eq, Hash, AsRef, From, FromStr, Into, Ord, PartialOrd)]
pub struct Variable(CompactString);

#[derive(Clone, Copy, Debug, Display)]
#[derive(PartialEq, Eq, Hash, From, FromStr, Into, Ord, PartialOrd)]
pub struct ArgList(u64);

/// Term tree element
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TermNode {
    /// Functional (operation) symbol
    Symbol(Symbol),
    /// Named parameter. Can be replaced during the unification procedure
    Param(Param),
    /// Variable symbol
    Variable(Variable),
    /// Rational constant
    Number(Decimal),
    /// Can be replaced with list of terms
    ArgList(ArgList),
}

impl TermNode {
    /// Get function symbol by name
    ///
    /// returns None if the specified symbol name is not found in the database
    #[inline]
    pub fn with_symbol_opt(name: &str) -> Option<Self> {
        Symbol::by_name(name).map(Self::Symbol)
    }

    /// Same as with_func_symbol_opt(arg).unwrap()
    #[inline]
    pub fn with_symbol(name: &str) -> Self {
        Self::with_symbol_opt(name).unwrap()
    }

    /// Create a constant symbol
    #[inline]
    pub fn with_number(number: impl Into<Decimal>) -> Self {
        Self::Number(number.into())
    }

    /// Get the contents of a function symbol
    ///
    /// returns None if the content is non a functional symbol
    #[inline]
    pub fn symbol(&self) -> Option<Symbol> {
        if let TermNode::Symbol(s) = self {
            return Some(s.clone());
        }
        None
    }

    /// Get the contents of a variable symbol
    ///
    /// returns None if the content is non a variable symbol
    #[inline]
    pub fn variable(&self) -> Option<&Variable> {
        if let TermNode::Variable(v) = &self {
            return Some(v);
        }
        None
    }

    /// Get the contents of a parameter
    ///
    /// returns None if the content is non a parameter
    #[inline]
    pub fn param(&self) -> Option<&Param> {
        if let TermNode::Param(p) = &self {
            return Some(p);
        }
        None
    }

    /// Get the contents of a constant
    ///
    /// returns None if the content is non a constant
    #[inline]
    pub fn number(&self) -> Option<&Decimal> {
        if let TermNode::Number(d) = &self {
            return Some(d);
        }
        None
    }

    #[inline]
    /// Get the contents of a placeholder
    ///
    /// returns None if the content is non a placeholder
    pub fn placeholder(&self) -> Option<ArgList> {
        if let TermNode::ArgList(p) = &self {
            return Some(*p);
        }
        None
    }

    #[inline]
    /// Check if content is symbol with the name
    pub fn is_symbol_name(&self, name: &str) -> bool {
        if let TermNode::Symbol(s) = self {
            return s == name;
        }

        false
    }

    #[inline]
    /// Check if content is constant with the value
    pub fn is_number_value(&self, value: &Decimal) -> bool {
        if let TermNode::Number(num) = &self {
            return num == value;
        }
        false
    }
}

impl Ord for TermNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Symbol < Param < Varible < Number < Placeholder
        match (self, other) {
            (TermNode::Symbol(id_l), TermNode::Symbol(id_r)) => id_l.cmp(id_r),
            (TermNode::Symbol(_), _) => Ordering::Less,

            (TermNode::Param(id_l), TermNode::Param(id_r)) => id_l.cmp(id_r),
            (TermNode::Param(_), TermNode::Symbol(_)) => Ordering::Greater,
            (TermNode::Param(_), _) => Ordering::Less,

            (TermNode::Variable(id_l), TermNode::Variable(id_r)) => id_l.cmp(id_r),
            (TermNode::Variable(_), TermNode::Number(_)) => Ordering::Less,
            (TermNode::Variable(_), TermNode::ArgList(_)) => Ordering::Less,
            (TermNode::Variable(_), _) => Ordering::Greater,

            (TermNode::Number(d1), TermNode::Number(d2)) => d1.cmp(d2),
            (TermNode::Number(_), TermNode::ArgList(_)) => Ordering::Less,
            (TermNode::Number(_), _) => Ordering::Greater,

            (TermNode::ArgList(_), TermNode::ArgList(_)) => Ordering::Equal,
            (TermNode::ArgList(_), _) => Ordering::Greater,
        }
    }
}

impl PartialOrd for TermNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for TermNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TermNode::Symbol(s) => {
                write!(f, "{}", s)
            }
            TermNode::Param(id) => write!(f, "{id}"),
            TermNode::Number(value) => {
                if value.is_negative() {
                    write!(f, "({value})")
                } else {
                    write!(f, "{value}")
                }
            }
            TermNode::Variable(id) => write!(f, "{id}"),
            TermNode::ArgList(_) => write!(f, ".."),
        }
    }
}

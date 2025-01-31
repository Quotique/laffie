use std::{fmt, hash::Hash, sync::Arc};

use derive_more::{AsRef, Display, From, FromStr, Into};
use trees::{Node, Tree};

use crate::{CompactString, Decimal, Signed};

use super::func::FuncSymbol;

#[derive(Clone, Debug, Display)]
#[derive(PartialEq, Eq, Hash, AsRef, From, FromStr, Into, Ord, PartialOrd)]
pub struct Param(CompactString);

#[derive(Clone, Debug, Display)]
#[derive(PartialEq, Eq, Hash, AsRef, From, FromStr, Into, Ord, PartialOrd)]
pub struct Variable(CompactString);

#[derive(Clone, Copy, Debug, Display)]
#[derive(PartialEq, Eq, Hash, From, FromStr, Into, Ord, PartialOrd)]
pub struct Placeholder(u64);

pub type SymbolNode = Node<Symbol>;
pub type SymbolTree = Tree<Symbol>;

/// Term tree element
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Symbol {
    /// Functional (operation) symbol
    FuncSymbol(Arc<FuncSymbol>),
    /// Named parameter. Can be replaced during the unification procedure
    Param(Param),
    /// Variable symbol
    Variable(Variable),
    /// Rational non-negative constant
    Number(Decimal),
    /// Link to another subtree
    Placeholder(Placeholder),
}

impl Symbol {
    /// Get function symbol by name
    ///
    /// returns None if the specified symbol name is not found in the database
    #[inline]
    pub fn with_func_symbol_opt(name: &str) -> Option<Self> {
        FuncSymbol::by_name(name).map(Self::FuncSymbol)
    }

    /// Same as with_func_symbol_opt(arg).unwrap()
    #[inline]
    pub fn with_func_symbol(name: &str) -> Self {
        Self::with_func_symbol_opt(name).unwrap()
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
    pub fn func_symbol(&self) -> Option<Arc<FuncSymbol>> {
        if let Symbol::FuncSymbol(s) = self {
            return Some(s.clone());
        }
        None
    }

    /// Get the contents of a variable symbol
    ///
    /// returns None if the content is non a variable symbol
    #[inline]
    pub fn variable(&self) -> Option<&Variable> {
        if let Symbol::Variable(v) = &self {
            return Some(v);
        }
        None
    }

    /// Get the contents of a parameter
    ///
    /// returns None if the content is non a parameter
    #[inline]
    pub fn param(&self) -> Option<&Param> {
        if let Symbol::Param(p) = &self {
            return Some(p);
        }
        None
    }

    /// Get the contents of a constant
    ///
    /// returns None if the content is non a constant
    #[inline]
    pub fn number(&self) -> Option<&Decimal> {
        if let Symbol::Number(d) = &self {
            return Some(d);
        }
        None
    }

    #[inline]
    /// Get the contents of a placeholder
    ///
    /// returns None if the content is non a placeholder
    pub fn placeholder(&self) -> Option<&Placeholder> {
        if let Symbol::Placeholder(p) = &self {
            return Some(p);
        }
        None
    }

    #[inline]
    /// Check if content is symbol with the name
    pub fn is_symbol_name(&self, name: &str) -> bool {
        if let Symbol::FuncSymbol(s) = self {
            return s.name == name;
        }

        false
    }

    #[inline]
    /// Check if content is constant with the value
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

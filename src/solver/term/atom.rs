use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
};

use derive_more::{AsRef, Display, From, FromStr, Into};
use serde_derive::{Deserialize, Serialize};

use super::symbol::Symbol;
use crate::{CompactString, Rational, Signed, number_to_string};

#[derive(PartialEq, Eq, Hash)]
#[derive(Ord, PartialOrd)]
#[derive(Clone, AsRef, From, FromStr, Into)]
#[derive(Debug, Display, Serialize, Deserialize)]
pub struct Param(CompactString);

#[derive(Clone, Debug)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Variable {
    name: CompactString,
}

impl Variable {
    /// The variable's name.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.name.as_str()
    }
}

impl PartialEq for Variable {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Variable {}

impl Hash for Variable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Ord for Variable {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for Variable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<CompactString> for Variable {
    fn from(name: CompactString) -> Self {
        Self { name }
    }
}

impl AsRef<CompactString> for Variable {
    fn as_ref(&self) -> &CompactString {
        &self.name
    }
}

impl FromStr for Variable {
    type Err = <CompactString as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { name: s.parse()? })
    }
}

#[derive(PartialEq, Eq, Hash)]
#[derive(Ord, PartialOrd)]
#[derive(Clone, Copy, AsRef, From, FromStr, Into)]
#[derive(Debug, Display, Serialize, Deserialize)]
pub struct ArgList(u64);

/// Term tree element
#[derive(Clone)]
#[derive(PartialEq, Eq, Hash)]
#[derive(Debug, Serialize, Deserialize)]
pub enum Atom {
    /// Functional (operation) symbol
    Symbol(Symbol),
    /// Named parameter. Can be replaced during the unification procedure
    Param(Param),
    /// Variable symbol
    Variable(Variable),
    /// Rational constant
    Number(#[serde(with = "crate::rational::serde_str")] Rational),
    /// Can be replaced with list of terms
    ArgList(ArgList),
}

#[inline]
pub fn param(p: impl AsRef<str>) -> Param {
    Param(p.as_ref().into())
}

#[inline]
pub fn var(v: impl AsRef<str>) -> Variable {
    Variable {
        name: v.as_ref().into(),
    }
}

impl From<Symbol> for Atom {
    fn from(value: Symbol) -> Self {
        Self::Symbol(value)
    }
}

impl From<Rational> for Atom {
    fn from(value: Rational) -> Self {
        Self::Number(value)
    }
}

impl Atom {
    /// Get the contents of a function symbol
    ///
    /// returns None if the content is non a functional symbol
    #[inline]
    pub fn symbol(&self) -> Option<Symbol> {
        if let Atom::Symbol(s) = self {
            return Some(s.clone());
        }
        None
    }

    /// Get the contents of a variable symbol
    ///
    /// returns None if the content is non a variable symbol
    #[inline]
    pub fn variable(&self) -> Option<&Variable> {
        if let Atom::Variable(v) = &self {
            return Some(v);
        }
        None
    }

    /// Get the contents of a parameter
    ///
    /// returns None if the content is non a parameter
    #[inline]
    pub fn param(&self) -> Option<&Param> {
        if let Atom::Param(p) = &self {
            return Some(p);
        }
        None
    }

    /// Get the contents of a constant
    ///
    /// returns None if the content is non a constant
    #[inline]
    pub fn number(&self) -> Option<&Rational> {
        if let Atom::Number(d) = &self {
            return Some(d);
        }
        None
    }

    #[inline]
    /// Get the contents of a placeholder
    ///
    /// returns None if the content is non a placeholder
    pub fn placeholder(&self) -> Option<ArgList> {
        if let Atom::ArgList(p) = &self {
            return Some(*p);
        }
        None
    }

    #[inline]
    /// Check if content is symbol with the name
    pub fn is_symbol_name(&self, name: &str) -> bool {
        if let Atom::Symbol(s) = self {
            return s == name;
        }

        false
    }

    #[inline]
    /// Check if content is constant with the value
    pub fn is_number_value(&self, value: &Rational) -> bool {
        if let Atom::Number(num) = &self {
            return num == value;
        }
        false
    }
}

impl Ord for Atom {
    fn cmp(&self, other: &Self) -> Ordering {
        // Symbol < Param < Variable < Number < Placeholder
        match (self, other) {
            (Atom::Symbol(id_l), Atom::Symbol(id_r)) => id_l.cmp(id_r),
            (Atom::Symbol(_), _) => Ordering::Less,

            (Atom::Param(id_l), Atom::Param(id_r)) => id_l.cmp(id_r),
            (Atom::Param(_), Atom::Symbol(_)) => Ordering::Greater,
            (Atom::Param(_), _) => Ordering::Less,

            (Atom::Variable(id_l), Atom::Variable(id_r)) => id_l.cmp(id_r),
            (Atom::Variable(_), Atom::Number(_)) => Ordering::Less,
            (Atom::Variable(_), Atom::ArgList(_)) => Ordering::Less,
            (Atom::Variable(_), _) => Ordering::Greater,

            (Atom::Number(d1), Atom::Number(d2)) => d1.cmp(d2),
            (Atom::Number(_), Atom::ArgList(_)) => Ordering::Less,
            (Atom::Number(_), _) => Ordering::Greater,

            (Atom::ArgList(_), Atom::ArgList(_)) => Ordering::Equal,
            (Atom::ArgList(_), _) => Ordering::Greater,
        }
    }
}

impl PartialOrd for Atom {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Atom::Symbol(s) => write!(f, "{s}"),
            Atom::Param(id) => write!(f, "{id}"),
            Atom::Number(value) => {
                let s = number_to_string(value);
                if value.is_negative() {
                    write!(f, "({s})")
                } else {
                    write!(f, "{s}")
                }
            }
            Atom::Variable(id) => write!(f, "{id}"),
            Atom::ArgList(_) => write!(f, ".."),
        }
    }
}

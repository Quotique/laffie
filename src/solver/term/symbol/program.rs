use std::{cmp, collections::HashMap, fmt};

use super::{container, FuncSymbolBuilder, Symbol};
use crate::{
    term::{Subterm, SubtermMut},
    CompactString, NormalizationLevel,
};

pub type TruthChecker = dyn Fn(Subterm) -> Truth + Sync + Send;
pub type Comparator = dyn Fn(Subterm, Subterm) -> cmp::Ordering + Send + Sync;
pub type Calculator = dyn Fn(&mut SubtermMut, NormalizationLevel) -> bool + Send + Sync;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Truth {
    True,
    False,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SymbolAttr {
    Infix,
    Display,
    Associative,
    Commutative,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolAttrValue {
    None,
    UInt(u64),
    UStr(String),
}

pub struct SymbolProgram {
    pub name:          CompactString,
    pub attrs:         HashMap<SymbolAttr, SymbolAttrValue>,
    pub arg_cmp:       Box<Comparator>,
    pub calculator:    Box<Calculator>,
    pub truth_checker: Box<TruthChecker>,
}

impl Truth {
    #[inline]
    pub fn is_true(&self) -> bool {
        self == &Truth::True
    }

    #[inline]
    pub fn reverse(&self) -> Truth {
        match self {
            Truth::True => Truth::False,
            Truth::False => Truth::True,
            Truth::Unknown => Truth::Unknown,
        }
    }
}

impl Default for SymbolProgram {
    fn default() -> Self {
        Self {
            name:          Default::default(),
            attrs:         Default::default(),
            arg_cmp:       Box::new(|l, r| l.data().cmp(r.data())),
            calculator:    Box::new(|_, _| false),
            truth_checker: Box::new(|_| Truth::Unknown),
        }
    }
}

impl fmt::Debug for SymbolProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ name: {}, attrs: {:?},   }}", self.name, self.attrs)
    }
}

impl SymbolProgram {
    #[inline]
    pub fn builder() -> FuncSymbolBuilder {
        FuncSymbolBuilder::default()
    }

    #[inline]
    pub fn register(self) -> Symbol {
        container::add_symbol_impl(&mut container::all_func_symbols().write(), self)
    }

    #[inline]
    pub fn add_with_name(symbols: &mut HashMap<CompactString, Self>, name: &str) {
        container::add_symbol_impl(symbols, SymbolProgram::builder().name(name).build());
    }

    #[inline]
    pub fn extend_attrs(&mut self, attrs: impl IntoIterator<Item = (SymbolAttr, SymbolAttrValue)>) {
        self.attrs.extend(attrs)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn by_name_test() {
        let program = &SymbolProgram::builder()
            .name("==")
            .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(500))
            .with_truth_checker(Box::new(|_| Truth::Unknown))
            .build();
        assert_eq!(program.name, "==");
        assert_eq!(
            program.attrs.get(&SymbolAttr::Infix),
            Some(&SymbolAttrValue::UInt(500))
        )
    }
}

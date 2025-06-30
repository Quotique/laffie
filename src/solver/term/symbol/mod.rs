pub mod base;
mod container;
mod program;

use std::fmt;

use parking_lot::{lock_api::MappedRwLockReadGuard, RawRwLock, RwLockReadGuard};

pub use program::{SymbolAttr, SymbolAttrValue, SymbolProgram, Truth};

use crate::{
    term::{Subterm, SubtermMut},
    CompactString, NormalizationLevel,
};

#[derive(Default, Clone)]
#[derive(Debug)]
#[derive(PartialOrd, Ord)]
#[derive(PartialEq, Eq, Hash)]
pub struct Symbol(CompactString);

impl Symbol {
    #[inline]
    pub fn by_name(name: &str) -> Option<Self> {
        container::all_func_symbols()
            .read()
            .get(&CompactString::from(name))
            .map(|x| Symbol(x.name.clone()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[inline]
    pub fn is_associative(&self) -> bool {
        self.program().attrs.contains_key(&SymbolAttr::Associative)
    }

    #[inline]
    pub fn is_commutative(&self) -> bool {
        self.program().attrs.contains_key(&SymbolAttr::Commutative)
    }

    pub fn display_weight(&self) -> Option<u64> {
        if let Some(SymbolAttrValue::UInt(v)) = self.program().attrs.get(&SymbolAttr::Infix) {
            return Some(*v);
        }
        None
    }

    pub fn check_truth(&self, node: Subterm) -> Truth {
        (self.program().truth_checker)(node)
    }

    #[inline]
    pub fn evaluate(&self, node: &mut SubtermMut, level: NormalizationLevel) -> bool {
        (self.program().calculator)(node, level)
    }

    #[inline]
    pub fn arg_order(&self, left: Subterm, right: Subterm) -> std::cmp::Ordering {
        (self.program().arg_cmp)(left, right)
    }

    fn program(&self) -> MappedRwLockReadGuard<'_, RawRwLock, SymbolProgram> {
        RwLockReadGuard::map(container::all_func_symbols().read(), |x| {
            x.get(&self.0).expect("symbol program must exists")
        })
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(SymbolAttrValue::UStr(s)) = self.program().attrs.get(&SymbolAttr::Display) {
            write!(f, "{s}")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl PartialEq<str> for Symbol {
    fn eq(&self, other: &str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<&str> for Symbol {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

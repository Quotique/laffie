pub mod base;
mod container;
mod program;

use std::fmt;

use container::all_func_symbols;
use parking_lot::{RawRwLock, RwLockReadGuard, lock_api::MappedRwLockReadGuard};
use serde_derive::{Deserialize, Serialize};

pub use program::{SymbolAttr, SymbolAttrValue, SymbolProgram, Truth, TruthCtx};

use crate::{
    CompactString, NormLevel,
    term::{TermMut, TermRef},
};

#[derive(Default, Clone)]
#[derive(PartialOrd, Ord)]
#[derive(PartialEq, Eq, Hash)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Symbol(CompactString);

#[inline]
pub fn try_sym(name: impl AsRef<str>) -> Option<Symbol> {
    all_func_symbols()
        .read()
        .get(&CompactString::from(name.as_ref()))
        .map(|x| Symbol(x.name.clone()))
}

/// Names of every registered symbol (base + loaded). For tooling; not hot.
pub fn symbol_names() -> Vec<CompactString> {
    all_func_symbols().read().keys().cloned().collect()
}

#[inline]
pub fn sym(name: impl AsRef<str>) -> Symbol {
    let symbol = all_func_symbols().read()[&CompactString::from(name.as_ref())]
        .name
        .clone();
    Symbol(symbol)
}

impl Symbol {
    #[inline]
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

    pub fn check_truth(&self, node: TermRef, ctx: TruthCtx) -> Truth {
        (self.program().truth_checker)(node, ctx)
    }

    #[inline]
    pub fn evaluate(&self, node: &mut TermMut, level: NormLevel) -> bool {
        (self.program().calculator)(node, level)
    }

    #[inline]
    pub fn arg_order(&self, left: TermRef, right: TermRef) -> std::cmp::Ordering {
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

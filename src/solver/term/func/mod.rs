pub mod base;
mod builder;
mod container;
mod truth;

use std::{cmp, collections::HashMap, fmt, hash, str::FromStr, sync::Arc};

use parking_lot::RwLock;

use super::{Subterm, SubtermMut};
use crate::{CompactString, NormalizationLevel};

pub use builder::FuncSymbolBuilder;
pub use truth::{TruthChecker, TruthResult};

type Comparator = dyn Fn(Subterm, Subterm) -> std::cmp::Ordering + Send + Sync;
type CalculatorSignature = dyn Fn(&mut SubtermMut, NormalizationLevel) -> bool + Send + Sync;

pub struct Ordering(Box<Comparator>);
pub struct Calculator(Box<CalculatorSignature>);

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

#[derive(Default)]
pub struct FuncSymbol {
    pub name:          CompactString,
    pub attrs:         RwLock<HashMap<SymbolAttr, SymbolAttrValue>>,
    pub arg_order:     Option<Ordering>,
    pub calculator:    Option<Calculator>,
    pub truth_checker: Option<TruthChecker>,
}

impl fmt::Debug for FuncSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ name: {}, attrs: {:?}, ordering: {}, calculator: {}, truth_checker: {} }}",
            self.name,
            self.attrs.read(),
            self.arg_order.is_some(),
            self.calculator.is_some(),
            self.truth_checker.is_some()
        )
    }
}

impl FuncSymbol {
    #[inline]
    pub fn builder() -> FuncSymbolBuilder {
        FuncSymbolBuilder::default()
    }

    #[inline]
    pub fn register(self) -> Arc<Self> {
        container::add_symbol_impl(&mut container::all_func_symbols().write(), self)
    }

    #[inline]
    pub fn by_name(name: &str) -> Option<Arc<Self>> {
        container::all_func_symbols()
            .read()
            .get(&CompactString::from_str(name).unwrap())
            .cloned()
    }

    #[inline]
    pub fn add_with_name(symbols: &mut HashMap<CompactString, Arc<Self>>, name: &str) {
        container::add_symbol_impl(symbols, FuncSymbol::builder().name(name).build());
    }

    #[inline]
    pub fn extend_attrs(&self, attrs: impl IntoIterator<Item = (SymbolAttr, SymbolAttrValue)>) {
        self.attrs.write().extend(attrs)
    }

    #[inline]
    pub fn is_associative(&self) -> bool {
        self.attrs.read().contains_key(&SymbolAttr::Associative)
    }

    #[inline]
    pub fn is_commutative(&self) -> bool {
        self.attrs.read().contains_key(&SymbolAttr::Commutative)
    }

    pub fn display_weight(&self) -> Option<u64> {
        if let Some(SymbolAttrValue::UInt(v)) = self.attrs.read().get(&SymbolAttr::Infix) {
            return Some(*v);
        }
        None
    }

    pub fn check_truth(&self, node: Subterm) -> TruthResult {
        if let Some(c) = self.truth_checker.as_ref() {
            c.0(node)
        } else {
            TruthResult::Unknown
        }
    }

    pub fn evaluate(&self, node: &mut SubtermMut, level: NormalizationLevel) -> bool {
        if let Some(c) = self.calculator.as_ref() {
            c.0(node, level)
        } else {
            false
        }
    }

    #[inline]
    pub fn arg_order(&self, left: Subterm, right: Subterm) -> Option<std::cmp::Ordering> {
        self.arg_order.as_ref().as_ref().map(|o| o.0(left, right))
    }
}

impl fmt::Display for FuncSymbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(SymbolAttrValue::UStr(s)) = self.attrs.read().get(&SymbolAttr::Display) {
            write!(f, "{s}")
        } else {
            write!(f, "{}", self.name)
        }
    }
}

impl PartialOrd for FuncSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.name.cmp(&other.name))
    }
}

impl Ord for FuncSymbol {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialEq for FuncSymbol {
    fn eq(&self, other: &FuncSymbol) -> bool {
        self.name == other.name
    }
}

impl Eq for FuncSymbol {}

impl<T: AsRef<str>> PartialEq<T> for FuncSymbol {
    fn eq(&self, other: &T) -> bool {
        self.name == other.as_ref()
    }
}

impl hash::Hash for FuncSymbol {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn by_name_test() {
        let sym = FuncSymbol::by_name(&String::from("==")).unwrap();
        assert_eq!(
            sym.as_ref(),
            &FuncSymbol::builder()
                .name("==")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(500))
                .with_truth_checker(Box::new(|_| TruthResult::Unknown))
                .build()
        )
    }
}

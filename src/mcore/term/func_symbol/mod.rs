mod builder;
mod truth;

use std::{cmp, collections::HashMap, fmt, hash};

use parking_lot::RwLock;

use macros::FuncAttr;

use crate::{CompactString, NormalizationLevel};

use super::TermNode;

pub use builder::FuncSymbolBuilder;
pub use truth::{TruthChecker, TruthResult};

type BoxedComparator = Box<dyn Fn(&TermNode, &TermNode) -> std::cmp::Ordering>;
type CalculatorSignature = dyn Fn(&mut TermNode, NormalizationLevel) -> bool;

#[derive(FuncAttr)]
pub struct Ordering(BoxedComparator);
#[derive(FuncAttr)]
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

#[derive(Debug, Default)]
pub struct FuncSymbol {
    pub name:          CompactString,
    pub attrs:         RwLock<HashMap<SymbolAttr, SymbolAttrValue>>,
    pub arg_order:     Option<Ordering>,
    pub calculator:    Option<Calculator>,
    pub truth_checker: Option<TruthChecker>,
}

impl FuncSymbol {
    pub fn builder() -> FuncSymbolBuilder {
        FuncSymbolBuilder::default()
    }

    pub fn extend_attrs(&self, attrs: impl IntoIterator<Item = (SymbolAttr, SymbolAttrValue)>) {
        self.attrs.write().extend(attrs)
    }

    pub fn display_weight(&self) -> Option<u64> {
        if let Some(SymbolAttrValue::UInt(v)) = self.attrs.read().get(&SymbolAttr::Infix) {
            return Some(*v);
        }
        None
    }

    pub fn check_truth(&self, node: &TermNode) -> TruthResult {
        if let Some(c) = self.truth_checker.as_ref() {
            c.0(node)
        } else {
            TruthResult::Unknown
        }
    }

    pub fn evaluate(&self, node: &mut TermNode, level: NormalizationLevel) -> bool {
        if let Some(c) = self.calculator.as_ref() {
            c.0(node, level)
        } else {
            false
        }
    }

    pub fn arg_order(&self, left: &TermNode, right: &TermNode) -> Option<std::cmp::Ordering> {
        self.arg_order.as_ref().as_ref().map(|o| o.0(left, right))
    }
}

impl fmt::Display for FuncSymbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(SymbolAttrValue::UStr(s)) = self.attrs.read().get(&SymbolAttr::Display) {
            return write!(f, "{s}");
        }
        write!(f, "{}", self.name)
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
    use crate::predefine::symbol_by_name;

    use super::*;

    #[test]
    fn by_name_test() {
        let sym = symbol_by_name(&String::from("==")).unwrap();
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

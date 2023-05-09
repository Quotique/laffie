use std::{collections::HashMap, fmt, sync::Arc};

use derive_builder::Builder;

use macros::FuncAttr;

use crate::{NormalizationLevel, SymbolId};

use super::term::StatementNode;

type BoxedComparator = Box<dyn Fn(&StatementNode, &StatementNode) -> std::cmp::Ordering>;
type CalculatorSignature = dyn Fn(&mut StatementNode, NormalizationLevel) -> bool;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TruthResult {
    True,
    False,
    Unknown,
}

#[derive(FuncAttr)]
pub struct Ordering(BoxedComparator);
#[derive(FuncAttr)]
pub struct Calculator(Box<CalculatorSignature>);
#[derive(FuncAttr)]
pub struct TruthChecker(Box<dyn Fn(&StatementNode) -> TruthResult>);

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

#[derive(Builder, Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    #[builder(default)]
    pub id:            SymbolId,
    #[builder(setter(into))]
    pub name:          String,
    #[builder(default)]
    pub attrs:         HashMap<SymbolAttr, SymbolAttrValue>,
    #[builder(default = "Arc::new(None)", setter(into))]
    pub arg_order:     Arc<Option<Ordering>>,
    #[builder(default = "Arc::new(None)", setter(into))]
    pub calculator:    Arc<Option<Calculator>>,
    #[builder(default = "Arc::new(None)", setter(into))]
    pub truth_checker: Arc<Option<TruthChecker>>,
}

impl TruthResult {
    #[inline]
    pub fn is_true(&self) -> bool {
        self == &TruthResult::True
    }

    #[inline]
    pub fn reverse(&self) -> TruthResult {
        match self {
            TruthResult::True => TruthResult::False,
            TruthResult::False => TruthResult::True,
            TruthResult::Unknown => TruthResult::Unknown,
        }
    }
}

impl SymbolBuilder {
    pub fn with_attr(&mut self, name: SymbolAttr, value: SymbolAttrValue) -> &mut Self {
        if self.attrs.is_none() {
            self.attrs = Some(HashMap::default());
        }
        self.attrs.as_mut().unwrap().insert(name, value);

        self
    }

    pub fn with_calculator(&mut self, calculator: Box<CalculatorSignature>) -> &mut Self {
        self.calculator = Some(Arc::new(Some(Calculator(calculator))));
        self
    }

    pub fn with_truth_checker(
        &mut self,
        truth_checker: Box<dyn Fn(&StatementNode) -> TruthResult>,
    ) -> &mut Self {
        self.truth_checker = Some(Arc::new(Some(TruthChecker(truth_checker))));
        self
    }

    pub fn with_ordering(&mut self, ordering: BoxedComparator) -> &mut Self {
        self.arg_order = Some(Arc::new(Some(Ordering(ordering))));
        self
    }
}

impl Symbol {
    pub fn builder() -> SymbolBuilder {
        SymbolBuilder::default()
    }

    pub fn display_weight(&self) -> Option<u64> {
        if let Some(SymbolAttrValue::UInt(v)) = self.attrs.get(&SymbolAttr::Infix) {
            return Some(*v);
        }
        None
    }

    pub fn check_truth(&self, node: &StatementNode) -> TruthResult {
        if let Some(c) = self.truth_checker.as_ref() {
            c.0(node)
        } else {
            TruthResult::Unknown
        }
    }

    pub fn evaluate(&self, node: &mut StatementNode, level: NormalizationLevel) -> bool {
        if let Some(c) = self.calculator.as_ref() {
            c.0(node, level)
        } else {
            false
        }
    }

    pub fn arg_order(
        &self,
        left: &StatementNode,
        right: &StatementNode,
    ) -> Option<std::cmp::Ordering> {
        self.arg_order.as_ref().as_ref().map(|o| o.0(left, right))
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(SymbolAttrValue::UStr(s)) = self.attrs.get(&SymbolAttr::Display) {
            return write!(f, "{s}");
        }
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use crate::predefine::{symbol_by_id, symbol_by_name};

    use super::*;

    #[test]
    fn by_id_test() {
        let sym = symbol_by_id(1.into()).unwrap();
        assert_eq!(
            sym,
            Symbol::builder()
                .id(1.into())
                .name("==")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(500))
                .with_truth_checker(Box::new(|_| TruthResult::Unknown))
                .build()
                .unwrap()
        )
    }

    #[test]
    fn by_name_test() {
        let sym = symbol_by_name(&String::from("==")).unwrap();
        assert_eq!(
            sym,
            Symbol::builder()
                .id(1.into())
                .name("==")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(500))
                .with_truth_checker(Box::new(|_| TruthResult::Unknown))
                .build()
                .unwrap()
        )
    }
}

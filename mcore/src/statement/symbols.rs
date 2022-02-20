use std::{collections::HashMap, fmt, sync::Arc};

use derive_builder::Builder;

use macros::FuncAttr;

use super::term::StatementNode;

#[derive(FuncAttr)]
pub struct Ordering(Box<dyn Fn(&StatementNode, &StatementNode) -> std::cmp::Ordering>);
#[derive(FuncAttr)]
pub struct Calculator(Box<dyn Fn(&mut StatementNode) -> bool>);
#[derive(FuncAttr)]
pub struct TruthChecker(Box<dyn Fn(&StatementNode) -> bool>);

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
    #[builder(default = "0")]
    pub id:            u64,
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

impl SymbolBuilder {
    pub fn with_attr(&mut self, name: SymbolAttr, value: SymbolAttrValue) -> &mut Self {
        if self.attrs.is_none() {
            self.attrs = Some(HashMap::default());
        }
        self.attrs.as_mut().unwrap().insert(name, value);

        self
    }

    pub fn with_calculator(
        &mut self,
        calculator: Box<dyn Fn(&mut StatementNode) -> bool>,
    ) -> &mut Self {
        self.calculator = Some(Arc::new(Some(Calculator(calculator))));
        self
    }

    pub fn with_truth_checker(
        &mut self,
        truth_checker: Box<dyn Fn(&StatementNode) -> bool>,
    ) -> &mut Self {
        self.truth_checker = Some(Arc::new(Some(TruthChecker(truth_checker))));
        self
    }

    pub fn with_ordering(
        &mut self,
        ordering: Box<dyn Fn(&StatementNode, &StatementNode) -> std::cmp::Ordering>,
    ) -> &mut Self {
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

    pub fn check_truth(&self, node: &StatementNode) -> bool {
        if let Some(c) = self.truth_checker.as_ref() {
            c.0(node)
        } else {
            false
        }
    }

    pub fn evaluate(&self, node: &mut StatementNode) -> bool {
        if let Some(c) = self.calculator.as_ref() {
            c.0(node)
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
            return write!(f, "{}", s);
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
        let sym = symbol_by_id(1).unwrap();
        assert_eq!(
            sym,
            Symbol::builder()
                .id(1)
                .name("==")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(500))
                .with_truth_checker(Box::new(|_| false))
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
                .id(1)
                .name("==")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(500))
                .with_truth_checker(Box::new(|_| false))
                .build()
                .unwrap()
        )
    }
}

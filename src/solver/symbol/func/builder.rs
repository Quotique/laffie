use std::str::FromStr;

use crate::CompactString;

use super::{
    BoxedComparator, Calculator, CalculatorSignature, FuncSymbol, Ordering, SymbolAttr,
    SymbolAttrValue, SymbolNode, TruthChecker, TruthResult,
};

#[derive(Default)]
pub struct FuncSymbolBuilder {
    sym: FuncSymbol,
}

impl FuncSymbolBuilder {
    pub fn build(self) -> FuncSymbol {
        if self.sym.name.is_empty() {
            panic!("bad symbol name: name can not be empty");
        }
        self.sym
    }

    pub fn name(mut self, name: impl AsRef<str>) -> Self {
        self.sym.name = CompactString::from_str(name.as_ref()).unwrap();
        self
    }

    pub fn with_attr(self, name: SymbolAttr, value: SymbolAttrValue) -> Self {
        self.sym.attrs.write().insert(name, value);
        self
    }

    pub fn with_calculator(mut self, calculator: Box<CalculatorSignature>) -> Self {
        self.sym.calculator = Some(Calculator(calculator));
        self
    }

    pub fn with_truth_checker(
        mut self,
        truth_checker: Box<dyn Fn(SymbolNode) -> TruthResult + Send + Sync>,
    ) -> Self {
        self.sym.truth_checker = Some(TruthChecker(truth_checker));
        self
    }

    pub fn with_ordering(mut self, ordering: BoxedComparator) -> Self {
        self.sym.arg_order = Some(Ordering(ordering));
        self
    }
}

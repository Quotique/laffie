use std::str::FromStr;

use crate::CompactString;

use super::{Calculator, Comparator, SymbolAttr, SymbolAttrValue, SymbolProgram, TruthChecker};

#[derive(Default)]
pub struct FuncSymbolBuilder {
    sym: SymbolProgram,
}

impl FuncSymbolBuilder {
    pub fn build(self) -> SymbolProgram {
        if self.sym.name.is_empty() {
            panic!("bad symbol name: name can not be empty");
        }
        self.sym
    }

    pub fn name(mut self, name: impl AsRef<str>) -> Self {
        self.sym.name = CompactString::from_str(name.as_ref()).unwrap();
        self
    }

    pub fn with_attr(mut self, name: SymbolAttr, value: SymbolAttrValue) -> Self {
        self.sym.attrs.insert(name, value);
        self
    }

    pub fn with_calculator(mut self, calculator: Box<Calculator>) -> Self {
        self.sym.calculator = calculator;
        self
    }

    pub fn with_truth_checker(mut self, truth_checker: Box<TruthChecker>) -> Self {
        self.sym.truth_checker = truth_checker;
        self
    }

    pub fn with_ordering(mut self, ordering: Box<Comparator>) -> Self {
        self.sym.arg_cmp = ordering;
        self
    }
}

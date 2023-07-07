use crate::statement::{
    symbols::{Symbol, SymbolAttr, SymbolAttrValue, TruthResult},
    term::StatementNode,
};

use super::compare_numbers;

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("==")
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(500))
        .with_truth_checker(Box::new(equal))
        .build()
        .unwrap()
}

pub fn equal(root: &StatementNode) -> TruthResult {
    if !root.data().is_symbol_name("==") {
        return TruthResult::Unknown;
    }

    match compare_numbers(root.front().unwrap(), root.back().unwrap()) {
        Some(std::cmp::Ordering::Equal) => TruthResult::True,
        Some(_) => TruthResult::False,
        _ => TruthResult::Unknown,
    }
}

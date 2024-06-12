use crate::term::{
    func_symbol::{FuncSymbol, TruthResult},
    TermNode,
};

use super::compare_numbers;

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("!=")
        .with_truth_checker(Box::new(inequal))
        .build()
}

pub fn inequal(root: &TermNode) -> TruthResult {
    if !root.data().is_symbol_name("!=") {
        return TruthResult::Unknown;
    }

    match compare_numbers(root.front().unwrap(), root.back().unwrap()) {
        Some(std::cmp::Ordering::Equal) => TruthResult::False,
        Some(_) => TruthResult::True,
        _ => TruthResult::Unknown,
    }
}

use crate::statement::{
    symbols::{Symbol, TruthResult},
    term::StatementNode,
};

use super::compare_numbers;

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("<=")
        .with_truth_checker(Box::new(less_or_equal))
        .build()
        .unwrap()
}

pub fn less_or_equal(root: &StatementNode) -> TruthResult {
    if !root.data().is_symbol_name("<=") {
        return TruthResult::Unknown;
    }

    match compare_numbers(root.front().unwrap(), root.back().unwrap()) {
        Some(std::cmp::Ordering::Greater) => TruthResult::False,
        Some(_) => TruthResult::True,
        _ => TruthResult::Unknown,
    }
}

use crate::statement::{
    symbols::{Symbol, TruthResult},
    term::StatementNode,
};

use super::compare_numbers;

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("<")
        .with_truth_checker(Box::new(less))
        .build()
        .unwrap()
}

pub fn less(root: &StatementNode) -> TruthResult {
    if !root.data().is_symbol_name("<") {
        return TruthResult::Unknown;
    }

    match compare_numbers(root.front().unwrap(), root.back().unwrap()) {
        Some(std::cmp::Ordering::Less) => TruthResult::True,
        Some(_) => TruthResult::False,
        _ => TruthResult::Unknown,
    }
}

use crate::term::{FuncSymbol, Subterm, TruthResult};

use super::compare_numbers;

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name(">")
        .with_truth_checker(Box::new(more))
        .build()
}

pub fn more(root: Subterm) -> TruthResult {
    if !root.data().is_symbol_name(">") {
        return TruthResult::Unknown;
    }

    match compare_numbers(root.first_arg().unwrap(), root.last_arg().unwrap()) {
        Some(std::cmp::Ordering::Greater) => TruthResult::True,
        Some(_) => TruthResult::False,
        _ => TruthResult::Unknown,
    }
}

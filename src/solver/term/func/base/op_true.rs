use crate::term::{FuncSymbol, Subterm, TruthResult};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("true")
        .with_truth_checker(Box::new(is_true))
        .build()
}

pub fn is_true(root: Subterm) -> TruthResult {
    if !root.data().is_symbol_name("true") {
        return TruthResult::Unknown;
    }

    TruthResult::True
}

use crate::term::{FuncSymbol, TermNode, TruthResult};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("true")
        .with_truth_checker(Box::new(is_true))
        .build()
}

pub fn is_true(root: &TermNode) -> TruthResult {
    if !root.data().is_symbol_name("true") {
        return TruthResult::Unknown;
    }

    TruthResult::True
}

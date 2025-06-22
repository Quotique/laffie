use crate::term::{FuncSymbol, SymbolNode, TruthResult};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("true")
        .with_truth_checker(Box::new(is_true))
        .build()
}

pub fn is_true(root: SymbolNode) -> TruthResult {
    if !root.data().is_symbol_name("true") {
        return TruthResult::Unknown;
    }

    TruthResult::True
}

use crate::statement::{
    symbols::{Symbol, TruthResult},
    term::StatementNode,
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("true")
        .with_truth_checker(Box::new(is_true))
        .build()
        .unwrap()
}

pub fn is_true(root: &StatementNode) -> TruthResult {
    if !root.data().is_symbol_name("true") {
        return TruthResult::Unknown;
    }

    TruthResult::True
}

use crate::statement::{
    symbols::{Symbol, TruthResult},
    term::StatementNode,
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("symbolic_eq")
        .with_truth_checker(Box::new(symbolic_eq))
        .build()
        .unwrap()
}

pub fn symbolic_eq(root: &StatementNode) -> TruthResult {
    if !root.data().is_symbol_name("symbolic_eq") {
        return TruthResult::Unknown;
    }

    if root.degree() != 2 {
        return TruthResult::Unknown;
    }

    if root.front().unwrap() == root.back().unwrap() {
        TruthResult::True
    } else {
        TruthResult::False
    }
}

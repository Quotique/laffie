use crate::term::{FuncSymbol, Subterm, TruthResult};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("symbolic_eq")
        .with_truth_checker(Box::new(symbolic_eq))
        .build()
}

pub fn symbolic_eq(root: Subterm) -> TruthResult {
    if !root.data().is_symbol_name("symbolic_eq") {
        return TruthResult::Unknown;
    }

    if root.degree() != 2 {
        return TruthResult::Unknown;
    }

    if root.first_arg().unwrap() == root.last_arg().unwrap() {
        TruthResult::True
    } else {
        TruthResult::False
    }
}

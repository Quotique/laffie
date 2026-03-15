use super::SymbolProgram;
use crate::term::{Term, TermRef, Truth};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "symbolic_eq".into(),
        truth_checker: Box::new(symbolic_eq),
        ..Default::default()
    }
}

pub fn symbolic_eq(root: TermRef) -> Truth {
    if !root.data().is_symbol_name("symbolic_eq") {
        return Truth::Unknown;
    }

    if root.degree() != 2 {
        return Truth::Unknown;
    }

    if root.first_arg().unwrap() == root.last_arg().unwrap() {
        Truth::True
    } else {
        Truth::False
    }
}

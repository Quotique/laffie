use super::SymbolProgram;
use crate::term::{Subterm, Truth};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "true".into(),
        truth_checker: Box::new(is_true),
        ..Default::default()
    }
}

pub fn is_true(root: Subterm) -> Truth {
    if !root.data().is_symbol_name("true") {
        return Truth::Unknown;
    }

    Truth::True
}

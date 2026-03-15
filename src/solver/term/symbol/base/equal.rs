use std::collections::HashMap;

use super::{SymbolProgram, compare_numbers};
use crate::term::{SymbolAttr, SymbolAttrValue, Term, TermRef, Truth};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "==".into(),
        attrs: HashMap::from([(SymbolAttr::Infix, SymbolAttrValue::UInt(500))]),
        truth_checker: Box::new(equal),
        ..Default::default()
    }
}

pub fn equal(root: TermRef) -> Truth {
    if !root.data().is_symbol_name("==") {
        return Truth::Unknown;
    }

    match compare_numbers(root.first_arg().unwrap(), root.last_arg().unwrap()) {
        Some(std::cmp::Ordering::Equal) => Truth::True,
        Some(_) => Truth::False,
        _ => Truth::Unknown,
    }
}

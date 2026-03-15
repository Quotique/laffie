use std::collections::HashMap;

use super::SymbolProgram;
use crate::term::{Atom, SymbolAttr, SymbolAttrValue, Term, TermRef, Truth};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "is".into(),
        attrs: HashMap::from([
            (SymbolAttr::Infix, SymbolAttrValue::UInt(800)),
            (SymbolAttr::Display, SymbolAttrValue::UStr(" is ".into())),
        ]),
        truth_checker: Box::new(is),
        ..Default::default()
    }
}

pub fn is(root: TermRef) -> Truth {
    if !root.data().is_symbol_name("is") {
        return Truth::Unknown;
    }

    if let (Atom::Number(_), Atom::Symbol(known)) = (
        &root.first_arg().unwrap().data(),
        &root.last_arg().unwrap().data(),
    ) {
        if known == "known" {
            return Truth::True;
        } else {
            return Truth::Unknown;
        }
    }
    if let (Atom::Variable(_), Atom::Symbol(sym_varible)) = (
        &root.first_arg().unwrap().data(),
        &root.last_arg().unwrap().data(),
    ) {
        if sym_varible == "variable" {
            return Truth::True;
        } else {
            return Truth::Unknown;
        }
    }

    Truth::Unknown
}

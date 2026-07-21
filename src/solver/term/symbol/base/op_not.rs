use super::SymbolProgram;
use crate::{
    NormLevel,
    term::{Atom, Term, TermMut, TermRef, Truth, sym},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "!".into(),
        calculator: Box::new(not_replace),
        truth_checker: Box::new(is_not),
        ..Default::default()
    }
}

fn not_replace(root: &mut TermMut, _: NormLevel) -> bool {
    if !root.data().is_symbol_name("!") || root.degree() != 1 {
        return false;
    }

    match root.first_arg().unwrap().data().symbol() {
        Some(name) if name == "==" => {
            let mut child = root.pop_first_arg().unwrap();
            root.swap(&mut child.term_mut());
            *root.data_mut() = Atom::from(sym("!="));
            true
        }
        Some(name) if name == "!=" => {
            let mut child = root.pop_first_arg().unwrap();
            root.swap(&mut child.term_mut());
            *root.data_mut() = Atom::from(sym("=="));
            true
        }
        _ => false,
    }
}

pub fn is_not(root: TermRef) -> Truth {
    if !root.data().is_symbol_name("!") {
        return Truth::Unknown;
    }

    if let Some(child) = root.first_arg() {
        return child.truth().reverse();
    }

    Truth::Unknown
}

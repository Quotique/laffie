use super::SymbolProgram;
use crate::{
    term::{Subterm, SubtermMut, TermNode, Truth},
    NormalizationLevel,
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "!".into(),
        calculator: Box::new(not_replace),
        truth_checker: Box::new(is_not),
        ..Default::default()
    }
}

fn not_replace(root: &mut SubtermMut, _: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("!") || root.degree() != 1 {
        return false;
    }

    match root.first_arg().unwrap().data().symbol() {
        Some(name) if name == "==" => {
            let mut child = root.pop_first_arg().unwrap();
            root.swap(&mut child.as_subterm_mut());
            *root.data_mut() = TermNode::with_symbol("!=");
            true
        }
        Some(name) if name == "!=" => {
            let mut child = root.pop_first_arg().unwrap();
            root.swap(&mut child.as_subterm_mut());
            *root.data_mut() = TermNode::with_symbol("==");
            true
        }
        _ => false,
    }
}

pub fn is_not(root: Subterm) -> Truth {
    if !root.data().is_symbol_name("!") {
        return Truth::Unknown;
    }

    if let Some(child) = root.first_arg() {
        return child.truth().reverse();
    }

    Truth::Unknown
}

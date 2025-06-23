use crate::{
    term::{FuncSymbol, Subterm, SubtermMut, Symbol, TruthResult},
    NormalizationLevel,
};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("!")
        .with_truth_checker(Box::new(is_not))
        .with_calculator(Box::new(not_replace))
        .build()
}

fn not_replace(root: &mut SubtermMut, _: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("!") || root.degree() != 1 {
        return false;
    }

    match root
        .first_arg()
        .unwrap()
        .data()
        .func_symbol()
        .map(|x| x.name.clone())
    {
        Some(name) if name == "==" => {
            let mut child = root.pop_first_arg().unwrap();
            root.swap(&mut child.as_subterm_mut());
            *root.data_mut() = Symbol::with_func_symbol("!=");
            true
        }
        Some(name) if name == "!=" => {
            let mut child = root.pop_first_arg().unwrap();
            root.swap(&mut child.as_subterm_mut());
            *root.data_mut() = Symbol::with_func_symbol("==");
            true
        }
        _ => false,
    }
}

pub fn is_not(root: Subterm) -> TruthResult {
    if !root.data().is_symbol_name("!") {
        return TruthResult::Unknown;
    }

    if let Some(child) = root.first_arg() {
        return child.truth().reverse();
    }

    TruthResult::Unknown
}

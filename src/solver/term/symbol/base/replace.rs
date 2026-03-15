use super::SymbolProgram;
use crate::{
    NormalizationLevel,
    term::{Term, TermBuf, TermMut, VariableSubstitution},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "replace".into(),
        calculator: Box::new(replace),
        ..Default::default()
    }
}

pub fn replace(root: &mut TermMut, _: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("replace") || root.degree() != 2 {
        return false;
    }

    if root.values().any(|x| x.param().is_some()) {
        return false;
    }

    let map = root
        .pop_first_arg()
        .expect("replace must have a first argument");
    let map = into_variable_map(map);

    let mut term = root
        .pop_first_arg()
        .expect("replace must have a second argument");

    term.term_mut().apply_variable_map(&map);

    root.swap(&mut term.term_mut());
    true
}

fn into_variable_map(mut state: TermBuf) -> VariableSubstitution {
    let mut result = VariableSubstitution::default();

    if !state.data().is_symbol_name("==") || state.term().degree() != 2 {
        return result;
    }
    let var = state.term().first_arg().expect("must be");

    if let Some(v) = var.data().variable() {
        result.insert(v.clone(), state.term_mut().pop_last_arg().unwrap());
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::{NormalizationLevel, term::term_with_vars};

    #[test]
    fn replace_test() {
        insta::assert_snapshot!(
          term_with_vars(r#"replace(x == 5, x^4 - 25*x^2 + 60*x -36 != 0)"#)
                .normalize(NormalizationLevel::max()),
            @"264!=0");
    }
}

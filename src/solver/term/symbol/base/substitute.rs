use super::SymbolProgram;
use crate::{
    NormLevel,
    term::{Term, TermBuf, TermMut, VariableSubstitution},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "substitute".into(),
        calculator: Box::new(substitute),
        ..Default::default()
    }
}

pub fn substitute(root: &mut TermMut, level: NormLevel) -> bool {
    if !root.data().is_symbol_name("substitute") || root.degree() != 2 {
        return false;
    }

    if root.values().any(|x| x.param().is_some()) {
        return false;
    }

    let map = root
        .pop_first_arg()
        .expect("substitute must have a first argument");
    let map = into_variable_map(map);

    let mut term = root
        .pop_first_arg()
        .expect("substitute must have a second argument");

    term.term_mut().apply_variable_map(&map);

    root.swap(&mut term.term_mut());
    root.normalize(level);
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
    use crate::{NormLevel, term::term_with_vars};

    #[test]
    fn substitute_test() {
        insta::assert_snapshot!(
          term_with_vars(r#"substitute(x == 5, x^4 - 25*x^2 + 60*x -36 != 0)"#)
                .normalize(NormLevel::Full),
            @"264!=0");
    }

    #[test]
    fn substitute_cubic_root_test() {
        let result = term_with_vars(r#"substitute(x == 1, x^3 - 6*x^2 + 11*x - 6) == 0"#)
            .normalize(NormLevel::Full);
        insta::assert_snapshot!(result, @"0==0");
    }

    #[test]
    fn normalize_after_substitute_test() {
        let result = term_with_vars(r#"1^3 - 6*1^2 + 11*1 - 6 == 0"#).normalize(NormLevel::Full);
        insta::assert_snapshot!(result, @"0==0");
    }
}

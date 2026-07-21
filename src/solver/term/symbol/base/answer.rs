use super::SymbolProgram;
use crate::{
    NormLevel,
    term::{Term, TermMut},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "answer".into(),
        calculator: Box::new(answer),
        ..Default::default()
    }
}

/// Collapses `answer(answer(...(X)))` of any depth to `answer(X)`.
pub fn answer(root: &mut TermMut, _: NormLevel) -> bool {
    if !root.data().is_symbol_name("answer") || root.degree() != 1 {
        return false;
    }
    let mut changed = false;
    while root
        .first_arg()
        .map(|f| f.data().is_symbol_name("answer") && f.degree() == 1)
        .unwrap_or(false)
    {
        let mut inner = root.pop_first_arg().unwrap();
        let x = inner.term_mut().pop_first_arg().unwrap();
        root.push_first_arg(x);
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use crate::{NormLevel, term::term_with_vars};

    #[test]
    fn answer_collapses_double() {
        let mut t = term_with_vars("answer(answer(x))");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "answer(x)");
    }

    #[test]
    fn answer_collapses_triple() {
        let mut t = term_with_vars("answer(answer(answer(x)))");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "answer(x)");
    }

    #[test]
    fn answer_no_op_on_single() {
        let mut t = term_with_vars("answer(x)");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "answer(x)");
    }
}

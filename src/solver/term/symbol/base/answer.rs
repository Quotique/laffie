use super::SymbolProgram;
use crate::{
    NormLevel,
    term::{Term, TermBuf, TermMut, TermRef},
};

const NAME: &str = "answer";

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: NAME.into(),
        calculator: Box::new(answer),
        ..Default::default()
    }
}

pub fn mark(inner: TermBuf) -> TermBuf {
    TermBuf::symbol(NAME).arg(inner)
}

pub fn marked(root: TermRef) -> Option<TermRef> {
    if !root.data().is_symbol_name(NAME) || root.degree() != 1 {
        return None;
    }
    root.first_arg()
}

/// Collapses `answer(answer(...(X)))` of any depth to `answer(X)`.
pub fn answer(root: &mut TermMut, _: NormLevel) -> bool {
    if marked(root.as_ref()).is_none() {
        return false;
    }
    let mut changed = false;
    while root.first_arg().and_then(marked).is_some() {
        let mut inner = root.pop_first_arg().unwrap();
        let x = inner.term_mut().pop_first_arg().unwrap();
        root.push_first_arg(x);
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::{NAME, mark, marked};
    use crate::{
        NormLevel,
        term::{TermBuf, term_with_vars},
    };

    #[test]
    fn only_a_one_argument_answer_is_a_marker() {
        assert!(marked(TermBuf::symbol(NAME).term()).is_none());
        assert!(marked(mark(TermBuf::one()).term()).is_some());
        let two_args = TermBuf::symbol(NAME)
            .arg(TermBuf::one())
            .arg(TermBuf::one());
        assert!(marked(two_args.term()).is_none());
    }

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

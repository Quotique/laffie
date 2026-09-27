use super::SymbolProgram;
use crate::{
    NormLevel, Signed,
    term::{Atom, TermMut},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "abs".into(),
        calculator: Box::new(abs),
        ..Default::default()
    }
}

/// Absolute value of a numeric literal; a symbolic argument stays.
pub fn abs(root: &mut TermMut, _: NormLevel) -> bool {
    if !root.data().is_symbol_name("abs") || root.degree() != 1 {
        return false;
    }

    let arg = root.pop_last_arg().unwrap();
    if let Atom::Number(r) = &arg.data() {
        *root.data_mut() = Atom::Number(r.abs());
        return true;
    }

    root.push_last_arg(arg);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::{Term, Truth, TruthCtx, term_with_vars};

    fn normalized(text: &'static str) -> String {
        let mut t = term_with_vars(text);
        t.term_mut().normalize(NormLevel::Full);
        t.term().to_string()
    }

    #[test]
    fn abs_of_negative_fraction() {
        assert_eq!(normalized("abs(-3/2)"), normalized("3/2"));
    }

    #[test]
    fn abs_of_positive_is_itself() {
        assert_eq!(normalized("abs(2)"), "2");
    }

    #[test]
    fn abs_of_symbolic_stays_unevaluated() {
        assert_eq!(normalized("abs(a)"), "abs(a)");
    }

    #[test]
    fn a_requirement_on_abs_of_a_literal_is_decided() {
        let mut t = term_with_vars("2 >= abs(-1)");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().truth(TruthCtx::default()), Truth::True);
    }
}

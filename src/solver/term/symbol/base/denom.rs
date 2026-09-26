use super::SymbolProgram;
use crate::{
    NormLevel, Rational,
    term::{Atom, TermMut},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "denom".into(),
        calculator: Box::new(denom),
        ..Default::default()
    }
}

/// Denominator of a rational literal. A symbolic argument stays, so
/// `denom(r) is even` is undecidable rather than false.
pub fn denom(root: &mut TermMut, _: NormLevel) -> bool {
    if !root.data().is_symbol_name("denom") || root.degree() != 1 {
        return false;
    }

    let arg = root.pop_last_arg().unwrap();
    if let Atom::Number(r) = &arg.data() {
        let d = Rational::from(r.denom().clone());
        *root.data_mut() = Atom::Number(d);
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
    fn denom_of_fraction() {
        assert_eq!(normalized("denom(1/2)"), "2");
        assert_eq!(normalized("denom(3/4)"), "4");
    }

    #[test]
    fn denom_of_integer_is_one() {
        assert_eq!(normalized("denom(5)"), "1");
    }

    #[test]
    fn denom_reduces_before_answering() {
        // 2/4 folds to 1/2 during normalization, so the denominator is 2.
        assert_eq!(normalized("denom(2/4)"), "2");
    }

    #[test]
    fn denom_of_negative_is_positive() {
        assert_eq!(normalized("denom(-1/2)"), "2");
    }

    #[test]
    fn denom_of_symbolic_stays_unevaluated() {
        assert_eq!(normalized("denom(a)"), "denom(a)");
    }

    fn even_denominator(text: &'static str) -> Truth {
        let mut t = term_with_vars(text);
        t.term_mut().normalize(NormLevel::Full);
        t.term().truth(TruthCtx::default())
    }

    #[test]
    fn even_denominator_predicate() {
        assert_eq!(even_denominator("denom(1/2) is even"), Truth::True);
        assert_eq!(even_denominator("denom(3/2) is even"), Truth::True);
        assert_eq!(even_denominator("denom(1/3) is even"), Truth::False);
        assert_eq!(even_denominator("denom(2) is even"), Truth::False);
        assert_eq!(even_denominator("denom(r) is even"), Truth::Unknown);
    }
}

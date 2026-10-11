use std::collections::HashMap;

use num::Integer;

use super::SymbolProgram;
use crate::term::{Atom, SymbolAttr, SymbolAttrValue, Term, TermRef, Truth, TruthCtx, match_term};

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

pub fn is(root: TermRef, ctx: TruthCtx) -> Truth {
    let Some((lhs, rhs)) = match_term!(root, "is"(lhs, rhs)) else {
        return Truth::Unknown;
    };
    match rhs.data() {
        Atom::Symbol(s) if s == "atom" => {
            if lhs.degree() == 0 {
                Truth::True
            } else {
                Truth::False
            }
        }
        // Known iff every leaf is a number or a variable named in the context.
        Atom::Symbol(s) if s == "known" => {
            if lhs.bfs().all(|v| match v.data {
                Atom::Variable(var) => ctx.is_known(var.as_str()),
                Atom::Param(_) | Atom::ArgList(_) => false,
                _ => true,
            }) {
                Truth::True
            } else {
                Truth::False
            }
        }
        // Syntactic like `number`: only an unbound param is undecidable.
        Atom::Symbol(s) if s == "variable" => match lhs.data() {
            Atom::Variable(_) => Truth::True,
            Atom::Param(_) => Truth::Unknown,
            _ => Truth::False,
        },
        // Unlike `even`, a symbolic operand is False — literalness is decidable.
        Atom::Symbol(s) if s == "number" => {
            if lhs.data().number().is_some() {
                Truth::True
            } else {
                Truth::False
            }
        }
        // Even iff lhs is an even integer literal; a non-integer number is not
        // even, a symbolic operand is undecidable.
        Atom::Symbol(s) if s == "even" => match lhs.data().number() {
            Some(n) if n.is_integer() => {
                if n.to_integer().is_even() {
                    Truth::True
                } else {
                    Truth::False
                }
            }
            Some(_) => Truth::False,
            None => Truth::Unknown,
        },
        _ => Truth::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::{
        NormLevel,
        term::{TermBuf, term_with_vars},
    };

    fn names(list: &[&str]) -> HashSet<crate::CompactString> {
        list.iter().map(|s| (*s).into()).collect()
    }

    #[test]
    fn is_atom_true_for_number() {
        let t = term_with_vars("5 is atom");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::True);
    }

    #[test]
    fn is_atom_true_for_variable() {
        let t = term_with_vars("x is atom");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::True);
    }

    #[test]
    fn is_atom_false_for_compound() {
        let t = term_with_vars("x^2 is atom");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn is_atom_false_for_sum() {
        let t = term_with_vars("a + b is atom");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn is_known_true_for_number() {
        let t = term_with_vars("3 is known");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::True);
    }

    #[test]
    fn is_known_false_for_unknown_variable() {
        let t = term_with_vars("x is known");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn is_known_false_for_unknown_compound() {
        let t = term_with_vars("x + 1 is known");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn is_known_true_for_known_compound() {
        let lhs = term_with_vars("a^2 - 4");
        let t = TermBuf::symbol("is").arg(lhs).arg(TermBuf::symbol("known"));
        let known = names(&["a"]);
        assert_eq!(is(t.term(), TruthCtx::new(&known)), Truth::True);
    }

    #[test]
    fn is_known_false_for_partially_unknown_compound() {
        let lhs = term_with_vars("a^2 - 4");
        let t = TermBuf::symbol("is").arg(lhs).arg(TermBuf::symbol("known"));
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn is_variable_true_for_variable() {
        let t = term_with_vars("x is variable");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::True);
    }

    #[test]
    fn is_variable_false_for_number() {
        let t = term_with_vars("5 is variable");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn is_variable_false_for_compound() {
        let t = term_with_vars("x^2 is variable");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn non_is_term_returns_unknown() {
        let t = term_with_vars("x == 5");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::Unknown);
    }

    #[test]
    fn is_number_true_for_integer() {
        let t = term_with_vars("5 is number");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::True);
    }

    /// A literal fraction folds into one number only after normalization.
    #[test]
    fn is_number_true_for_normalized_fraction() {
        let mut t = term_with_vars("1/2 is number");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::True);
    }

    #[test]
    fn is_number_false_for_variable() {
        let t = term_with_vars("x is number");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn is_number_false_for_compound() {
        let t = term_with_vars("x + 1 is number");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn is_even_true_for_even_integer() {
        let t = term_with_vars("4 is even");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::True);
    }

    #[test]
    fn is_even_false_for_odd_integer() {
        let t = term_with_vars("3 is even");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn is_even_false_for_non_integer() {
        let t = term_with_vars("2.5 is even");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::False);
    }

    #[test]
    fn is_even_unknown_for_symbolic() {
        let t = term_with_vars("x is even");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::Unknown);
    }
}

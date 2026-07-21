use std::collections::HashMap;

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
        Atom::Symbol(s) if s == "variable" && matches!(lhs.data(), Atom::Variable(_)) => {
            Truth::True
        }
        _ => Truth::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::term::{TermBuf, term_with_vars};

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
    fn is_variable_unknown_for_number() {
        let t = term_with_vars("5 is variable");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::Unknown);
    }

    #[test]
    fn is_variable_unknown_for_compound() {
        let t = term_with_vars("x^2 is variable");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::Unknown);
    }

    #[test]
    fn non_is_term_returns_unknown() {
        let t = term_with_vars("x == 5");
        assert_eq!(is(t.term(), TruthCtx::default()), Truth::Unknown);
    }
}

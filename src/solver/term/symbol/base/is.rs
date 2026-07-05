use std::collections::HashMap;

use super::SymbolProgram;
use crate::term::{Atom, SymbolAttr, SymbolAttrValue, Term, TermRef, Truth, match_term};

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

pub fn is(root: TermRef) -> Truth {
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
        // Known iff every leaf is a number or a variable stamped known.
        Atom::Symbol(s) if s == "known" => {
            if lhs.bfs().all(|v| match v.data {
                Atom::Variable(var) => var.known,
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
    use super::*;
    use crate::term::{TermBuf, TermMut, term_with_vars};

    fn stamp(mut t: TermBuf, known: &[&str]) -> TermBuf {
        fn go(mut node: TermMut<'_>, known: &[&str]) {
            if let Atom::Variable(v) = node.data_mut() &&
                known.contains(&v.as_str())
            {
                v.known = true;
            }
            for child in node.iter_mut() {
                go(child, known);
            }
        }
        go(t.term_mut(), known);
        t
    }

    #[test]
    fn is_atom_true_for_number() {
        let t = term_with_vars("5 is atom");
        assert_eq!(is(t.term()), Truth::True);
    }

    #[test]
    fn is_atom_true_for_variable() {
        let t = term_with_vars("x is atom");
        assert_eq!(is(t.term()), Truth::True);
    }

    #[test]
    fn is_atom_false_for_compound() {
        let t = term_with_vars("x^2 is atom");
        assert_eq!(is(t.term()), Truth::False);
    }

    #[test]
    fn is_atom_false_for_sum() {
        let t = term_with_vars("a + b is atom");
        assert_eq!(is(t.term()), Truth::False);
    }

    #[test]
    fn is_known_true_for_number() {
        let t = term_with_vars("3 is known");
        assert_eq!(is(t.term()), Truth::True);
    }

    #[test]
    fn is_known_false_for_unknown_variable() {
        let t = term_with_vars("x is known");
        assert_eq!(is(t.term()), Truth::False);
    }

    #[test]
    fn is_known_false_for_unknown_compound() {
        let t = term_with_vars("x + 1 is known");
        assert_eq!(is(t.term()), Truth::False);
    }

    #[test]
    fn is_known_true_for_known_compound() {
        let lhs = stamp(term_with_vars("a^2 - 4"), &["a"]);
        let t = TermBuf::symbol("is").arg(lhs).arg(TermBuf::symbol("known"));
        assert_eq!(is(t.term()), Truth::True);
    }

    #[test]
    fn is_known_false_for_partially_unknown_compound() {
        let lhs = term_with_vars("a^2 - 4");
        let t = TermBuf::symbol("is").arg(lhs).arg(TermBuf::symbol("known"));
        assert_eq!(is(t.term()), Truth::False);
    }

    #[test]
    fn is_variable_true_for_variable() {
        let t = term_with_vars("x is variable");
        assert_eq!(is(t.term()), Truth::True);
    }

    #[test]
    fn is_variable_unknown_for_number() {
        let t = term_with_vars("5 is variable");
        assert_eq!(is(t.term()), Truth::Unknown);
    }

    #[test]
    fn is_variable_unknown_for_compound() {
        let t = term_with_vars("x^2 is variable");
        assert_eq!(is(t.term()), Truth::Unknown);
    }

    #[test]
    fn non_is_term_returns_unknown() {
        let t = term_with_vars("x == 5");
        assert_eq!(is(t.term()), Truth::Unknown);
    }
}

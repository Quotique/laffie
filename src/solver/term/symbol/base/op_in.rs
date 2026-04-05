use std::collections::HashMap;

use super::SymbolProgram;
use crate::term::{SymbolAttr, SymbolAttrValue, Term, TermRef, Truth, match_term};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "in".into(),
        attrs: HashMap::from([
            (SymbolAttr::Infix, SymbolAttrValue::UInt(100)),
            (SymbolAttr::Display, SymbolAttrValue::UStr("∈ ".into())),
        ]),
        truth_checker: Box::new(member_of),
        ..Default::default()
    }
}

/// Checks `a in set(...)`: true if `a` equals any element of the set.
fn member_of(root: TermRef) -> Truth {
    let Some((lhs, set_node)) = match_term!(root, "in"(lhs, "set"(set_node))) else {
        return Truth::Unknown;
    };

    if lhs
        .bfs()
        .any(|v| v.data.param().is_some() || v.data.variable().is_some())
    {
        return Truth::Unknown;
    }

    for element in set_node.args_iter() {
        if element == lhs {
            return Truth::True;
        }
    }

    Truth::False
}

#[cfg(test)]
mod tests {
    use crate::term::{Term, term_with_vars};

    use super::*;

    #[test]
    fn member_true() {
        let term = term_with_vars("3 in set(1, 2, 3)");
        assert_eq!(member_of(term.term()), Truth::True);
    }

    #[test]
    fn member_false() {
        let term = term_with_vars("5 in set(1, 2, 3)");
        assert_eq!(member_of(term.term()), Truth::False);
    }

    #[test]
    fn member_negative() {
        let term = term_with_vars("-2 in set(1, -2, 3)");
        assert_eq!(member_of(term.term()), Truth::True);
    }

    #[test]
    fn member_unknown_with_variable() {
        let term = term_with_vars("x in set(1, 2, 3)");
        assert_eq!(member_of(term.term()), Truth::Unknown);
    }

    #[test]
    fn member_not_set() {
        let term = term_with_vars("3 in empty_set");
        assert_eq!(member_of(term.term()), Truth::Unknown);
    }
}

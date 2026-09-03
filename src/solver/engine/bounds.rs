use crate::{
    Rational,
    term::{Term, TermRef},
};

/// True when `derived` proves `goal`: same expression, same bound kind, and a
/// bound at least as tight. `x > 2` proves `x > 0`.
pub(super) fn bound_implies(derived: TermRef, goal: TermRef) -> bool {
    let Some((de, dk, dc)) = as_bound(derived) else {
        return false;
    };
    let Some((ge, gk, gc)) = as_bound(goal) else {
        return false;
    };
    if de != ge {
        return false;
    }
    match (dk, gk) {
        (BoundKind::Lower { strict: ds }, BoundKind::Lower { strict: gs }) => {
            dc > gc || (dc == gc && (ds || !gs))
        }
        (BoundKind::Upper { strict: ds }, BoundKind::Upper { strict: gs }) => {
            dc < gc || (dc == gc && (ds || !gs))
        }
        _ => false,
    }
}

/// Which side of an expression a number bounds, and how tightly.
enum BoundKind {
    Lower { strict: bool },
    Upper { strict: bool },
}

/// Splits a comparison into (expression, bound kind, number). `E > c` gives a
/// lower bound, `c > E` an upper one. `None` unless exactly one side is a
/// number.
fn as_bound<'a>(t: TermRef<'a>) -> Option<(TermRef<'a>, BoundKind, Rational)> {
    if t.degree() != 2 {
        return None;
    }
    let data = t.data();
    let strict = if data.is_symbol_name(">") || data.is_symbol_name("<") {
        true
    } else if data.is_symbol_name(">=") || data.is_symbol_name("<=") {
        false
    } else {
        return None;
    };
    let greaterish = data.is_symbol_name(">") || data.is_symbol_name(">=");

    let lhs = t.first_arg()?;
    let rhs = t.last_arg()?;
    if let Some(c) = rhs.data().number() {
        let kind = if greaterish {
            BoundKind::Lower { strict }
        } else {
            BoundKind::Upper { strict }
        };
        Some((lhs, kind, c.clone()))
    } else if let Some(c) = lhs.data().number() {
        let kind = if greaterish {
            BoundKind::Upper { strict }
        } else {
            BoundKind::Lower { strict }
        };
        Some((rhs, kind, c.clone()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::bound_implies;
    use crate::term::term_with_vars;

    fn implies(derived: &'static str, goal: &'static str) -> bool {
        bound_implies(term_with_vars(derived).term(), term_with_vars(goal).term())
    }

    #[test]
    fn a_tighter_lower_bound_implies_a_looser_one() {
        assert!(implies("x > 2", "x > 0"));
        assert!(implies("x >= 2", "x > 0"));
        assert!(!implies("x > 2", "x > 3"));
    }

    #[test]
    fn a_tighter_upper_bound_implies_a_looser_one() {
        assert!(implies("x <= -1", "x < 0"));
        assert!(implies("x < -1", "x < 0"));
        assert!(!implies("x < 5", "x < -1"));
    }

    #[test]
    fn at_an_equal_bound_strict_implies_non_strict_but_not_the_reverse() {
        assert!(implies("x > 0", "x >= 0"));
        assert!(implies("x > 0", "x > 0"));
        assert!(implies("x >= 0", "x >= 0"));
        assert!(!implies("x >= 0", "x > 0"));
    }

    #[test]
    fn a_number_on_the_left_flips_the_bound() {
        assert!(implies("0 < x", "x > 0"));
        assert!(implies("x > 2", "0 < x"));
        assert!(implies("2 < x", "0 < x"));
    }

    #[test]
    fn bounds_of_different_kinds_imply_nothing() {
        assert!(!implies("x < 5", "x > 0"));
        assert!(!implies("x > 0", "x < 5"));
    }

    #[test]
    fn bounds_on_different_expressions_imply_nothing() {
        assert!(!implies("x > 2", "y > 0"));
        assert!(implies("x + y > 2", "x + y > 0"));
    }

    #[test]
    fn a_term_that_is_not_a_numeric_comparison_is_no_bound() {
        assert!(!implies("x == 2", "x > 0"));
        assert!(!implies("x > y", "x > 0"));
        assert!(!implies("x > 2", "x == 0"));
    }
}

use std::collections::HashMap;

use num::{One, Zero};

use super::{
    SymbolProgram,
    mul::{extract_numeric_factor, prepend_factor},
};
use crate::{
    NormLevel,
    term::{Atom, SymbolAttr, SymbolAttrValue, Term, TermBuf, TermMut},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "/".into(),
        attrs: HashMap::from([(SymbolAttr::Infix, SymbolAttrValue::UInt(200))]),
        calculator: Box::new(divide),
        ..Default::default()
    }
}

pub fn divide(root: &mut TermMut, level: NormLevel) -> bool {
    if !root.data().is_symbol_name("/") {
        return false;
    }

    match level {
        NormLevel::Off => false,
        NormLevel::Units => {
            if let Atom::Number(d) = &root.last_arg().unwrap().data() &&
                d.is_one()
            {
                let mut child = root.pop_first_arg().unwrap();
                root.swap(&mut child.term_mut());
                return true;
            }
            false
        }
        NormLevel::ConstFold | NormLevel::Full => {
            let num = root.first_arg().unwrap().data().number().cloned();
            let den = root.last_arg().unwrap().data().number().cloned();
            match (num, den) {
                // Number / Number: rationals are closed under division, so this
                // always collapses to a single reduced number.
                (Some(n), Some(d)) => {
                    if d.is_zero() {
                        return false;
                    }
                    root.swap(&mut TermBuf::ratio(n / d).term_mut());
                    true
                }
                (_, Some(d)) if d.is_one() => {
                    let mut child = root.pop_first_arg().unwrap();
                    root.swap(&mut child.term_mut());
                    true
                }
                (_, Some(d)) if d.is_zero() => false,
                // Product / Number: fold the divisor into the product's numeric
                // coefficient (exact — no leftover denominator).
                (_, Some(d)) => {
                    let first_owned = root.first_arg().unwrap().to_owned();
                    let Some((c, rest)) = extract_numeric_factor(first_owned.term()) else {
                        return false;
                    };
                    let new_c = c / d;
                    let mut new_root = if new_c.is_one() {
                        rest
                    } else {
                        prepend_factor(TermBuf::ratio(new_c), rest)
                    };
                    root.swap(&mut new_root.term_mut());
                    true
                }
                (_, _) => false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::term_with_vars;

    /// Runs `divide` once at `level`, asserting its change flag and the result.
    fn check(source: &'static str, level: NormLevel, changed: bool, expected: &str) {
        let mut t = term_with_vars(source);
        assert_eq!(
            divide(&mut t.term_mut(), level),
            changed,
            "change flag for {source} at {level:?}"
        );
        assert_eq!(t.to_string(), expected, "result of {source} at {level:?}");
    }

    #[test]
    fn off_is_noop() {
        for source in ["2/3", "2/1", "(6*a)/4"] {
            let rendered = term_with_vars(source).to_string();
            check(source, NormLevel::Off, false, &rendered);
        }
    }

    #[test]
    fn units_only_drops_denominator_one() {
        check("2/1", NormLevel::Units, true, "2");
        check("a/1", NormLevel::Units, true, "a");
        // A non-unit denominator is not folded at `Units`.
        check("2/3", NormLevel::Units, false, "2/3");
        check("(6*a)/4", NormLevel::Units, false, "(6*a)/4");
    }

    /// Rationals are exact: a number division collapses to a single reduced
    /// number (no leftover `/`), and a product over a number folds into the
    /// coefficient.
    #[test]
    fn const_fold_and_full_fold_numbers() {
        for level in [NormLevel::ConstFold, NormLevel::Full] {
            check("2/3", level, true, "2/3");
            check("25/35", level, true, "5/7");
            check("2.5/3.5", level, true, "5/7");
            check("(-10)/6", level, true, "-5/3");
            check("(2*a)/(-2)", level, true, "-a");
            check("(6*a)/4", level, true, "1.5*a");
            check("(2*a)/3", level, true, "2/3*a");
            check("(2*a*b)/(-2)", level, true, "-a*b");
        }
    }
}

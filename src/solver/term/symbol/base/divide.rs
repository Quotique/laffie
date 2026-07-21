use std::{cmp::max, collections::HashMap};

use bigdecimal::{BigDecimal as Decimal, One, Zero};
use num::{Signed, integer::gcd};
use num_bigint::ToBigInt;

use super::{
    MAX_DEC_CONVERSION_EXP, SymbolProgram,
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
            match (
                root.first_arg().unwrap().data().number(),
                root.last_arg().unwrap().data().number(),
            ) {
                (Some(d1), Some(d2)) => {
                    let (num, den) = simplify(d1.clone(), d2.clone());
                    if let Some(t) = impl_divide(&num, &den) {
                        root.swap(&mut TermBuf::number(t).term_mut());
                        true
                    } else if *d1 != num || *d2 != den {
                        let num = num * den.signum();
                        root.first_arg_mut()
                            .unwrap()
                            .swap(&mut TermBuf::number(num).term_mut());
                        *root.last_arg_mut().unwrap().data_mut() = Atom::Number(den.abs());
                        true
                    } else {
                        false
                    }
                }
                (_, Some(d2)) if d2.is_one() => {
                    let mut child = root.pop_first_arg().unwrap();
                    root.swap(&mut child.term_mut());
                    true
                }
                (_, Some(d2)) if d2.is_zero() => false,
                (_, Some(d2)) => {
                    let first_owned = root.first_arg().unwrap().to_owned();
                    let Some((c, rest)) = extract_numeric_factor(first_owned.term()) else {
                        return false;
                    };
                    let (num, den) = simplify(c.clone(), d2.clone());
                    if num == c && den == *d2 {
                        return false;
                    }
                    let signed_num = num * den.signum();
                    let den_abs = den.abs();

                    let new_num = if signed_num.is_one() {
                        rest
                    } else {
                        prepend_factor(TermBuf::number(signed_num), rest)
                    };
                    let mut new_root = if den_abs.is_one() {
                        new_num
                    } else {
                        TermBuf::symbol("/")
                            .arg(new_num)
                            .arg(TermBuf::number(den_abs))
                    };
                    root.swap(&mut new_root.term_mut());
                    true
                }
                (_, _) => false,
            }
        }
    }
}

fn simplify(num: Decimal, den: Decimal) -> (Decimal, Decimal) {
    let (num_m, num_e) = num.into_bigint_and_exponent();
    let (den_m, den_e) = den.into_bigint_and_exponent();

    let (num_e, den_e) = (num_e - max(num_e, den_e), den_e - max(num_e, den_e));
    let (num_m, den_m) = (
        num_m *
            Decimal::new(10.into(), num_e)
                .to_bigint()
                .expect("Unable to get bigint"),
        den_m *
            Decimal::new(10.into(), den_e)
                .to_bigint()
                .expect("Unable to get bigint"),
    );

    let g = gcd(num_m.clone(), den_m.clone());
    (Decimal::from(num_m / g.clone()), Decimal::from(den_m / g))
}

fn impl_divide(num: &Decimal, den: &Decimal) -> Option<Decimal> {
    if den.is_zero() {
        return None;
    }
    if den.is_one() {
        Some(num.clone())
    } else {
        let res = (num / den).normalized();
        let (_, exp) = res.clone().into_bigint_and_exponent();
        if exp <= MAX_DEC_CONVERSION_EXP {
            Some(res)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::symbol::base::calculator_check;

    #[test]
    fn derive_test() {
        assert_eq!(impl_divide(&5.into(), &1.into()), Some(5.into()));
        assert_eq!(
            impl_divide(&5.into(), &2.into()),
            Some(Decimal::new(25.into(), 1))
        );
        assert_eq!(impl_divide(&1.into(), &3.into()), None);
    }

    #[test]
    fn simplify_test() {
        assert_eq!(simplify(100.into(), 20.into()), (5.into(), 1.into()));

        assert_eq!(
            simplify(Decimal::new(35.into(), 1), Decimal::new(14.into(), 1)),
            (5.into(), 2.into())
        );
    }

    #[test]
    fn simplify_test_negative() {
        assert_eq!(simplify((-4).into(), 6.into()), ((-2).into(), 3.into()));
        assert_eq!(simplify(4.into(), (-6).into()), (2.into(), (-3).into()));
        assert_eq!(
            simplify((-4).into(), (-6).into()),
            ((-2).into(), (-3).into())
        );
    }

    #[test]
    fn calculator_test() {
        for (source, level_one, level_all) in [
            ("2/3", "2/3", "2/3"),
            ("2/1", "2", "2"),
            ("a/1", "a", "a"),
            ("25/35", "25/35", "5 / 7"),
            ("2.5/3.5", "2.5/3.5", "5 / 7"),
            ("(-10)/6", "(-10)/6", "(-5)/3"),
            ("(2*a)/(-2)", "(2*a)/(-2)", "(-1)*a"),
            ("(6*a)/4", "(6*a)/4", "(3*a)/2"),
            ("(2*a)/3", "(2*a)/3", "(2*a)/3"),
            ("(2*a*b)/(-2)", "(2*a*b)/(-2)", "(-1)*a*b"),
        ] {
            calculator_check(source, source, divide, NormLevel::Off);
            calculator_check(source, level_one, divide, NormLevel::Units);
            calculator_check(source, level_all, divide, NormLevel::Full);
        }
    }
}

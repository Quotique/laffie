use std::cmp::max;

use bigdecimal::{BigDecimal as Decimal, One, Zero};
use num::integer::gcd;
use num_bigint::ToBigInt;

use crate::{
    term::{FuncSymbol, SubtermMut, Symbol, SymbolAttr, SymbolAttrValue, Term},
    NormalizationLevel,
};

use super::MAX_DEC_CONVERSION_EXP;

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("/")
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(200))
        .with_calculator(Box::new(divide))
        .build()
}

pub fn divide(root: &mut SubtermMut, level: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("/") {
        return false;
    }

    match level {
        NormalizationLevel(0) => false,
        NormalizationLevel(1) => {
            if let Symbol::Number(d) = &root.last_arg().unwrap().data() {
                if d.is_one() {
                    let mut child = root.pop_first_arg().unwrap();
                    root.swap(&mut child.as_subterm_mut());
                    return true;
                }
            }
            false
        }
        _ => {
            match (
                root.first_arg().unwrap().data().number(),
                root.last_arg().unwrap().data().number(),
            ) {
                (Some(d1), Some(d2)) => {
                    let (num, den) = simplify(d1.clone(), d2.clone());
                    if let Some(t) = impl_divide(&num, &den) {
                        root.swap(&mut Term::number(t).as_subterm_mut());
                        true
                    } else if *d1 != num || *d2 != den {
                        let sign = if (&num * &den) < Decimal::zero() {
                            Decimal::from(-1)
                        } else {
                            Decimal::one()
                        };

                        root.first_arg_mut()
                            .unwrap()
                            .swap(&mut Term::number(sign * num.abs()).as_subterm_mut());
                        *root.last_arg_mut().unwrap().data_mut() = Symbol::Number(den.abs());
                        true
                    } else {
                        false
                    }
                }
                (_, Some(d)) => {
                    if d.is_one() {
                        let mut child = root.pop_first_arg().unwrap();
                        root.swap(&mut child.as_subterm_mut());
                        true
                    } else {
                        false
                    }
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
    use crate::term::func::base::calculator_check;

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
    fn calculator_test() {
        for (source, level_one, level_all) in [
            ("2/3", "2/3", "2/3"),
            ("2/1", "2", "2"),
            ("a/1", "a", "a"),
            ("25/35", "25/35", "5 / 7"),
            ("2.5/3.5", "2.5/3.5", "5 / 7"),
            ("(-10)/6", "(-10)/6", "(-5)/3"),
        ] {
            calculator_check(source, source, divide, NormalizationLevel(0));
            calculator_check(source, level_one, divide, NormalizationLevel(1));
            calculator_check(source, level_all, divide, NormalizationLevel::max());
        }
    }
}

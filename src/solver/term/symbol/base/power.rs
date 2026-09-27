use std::collections::HashMap;

use num::{BigInt, One, Signed, ToPrimitive, Zero, traits::Pow};

use super::SymbolProgram;
use crate::{
    NormLevel, Rational,
    term::{SymbolAttr, SymbolAttrValue, Term, TermBuf, TermMut, TermRef},
};

/// Maximum absolute integer exponent folded eagerly; larger powers are left
/// unevaluated to keep the numerator/denominator from blowing up.
const MAX_POWER_EXPONENT: i64 = 128;

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "^".into(),
        attrs: HashMap::from([(SymbolAttr::Infix, SymbolAttrValue::UInt(100))]),
        calculator: Box::new(power),
        ..Default::default()
    }
}

pub fn power(root: &mut TermMut, level: NormLevel) -> bool {
    if !root.data().is_symbol_name("^") {
        return false;
    }

    if level == NormLevel::Off {
        return false;
    }

    let exp = root.last_arg().unwrap().data().number().cloned();
    if let Some(e) = &exp &&
        let Some(mut canonical) = root_power(root.first_arg().unwrap(), e, level)
    {
        root.swap(&mut canonical.term_mut());
        return true;
    }

    let base = root.first_arg().unwrap().data().number().cloned();
    match (base, exp) {
        (Some(b), Some(e)) if level > NormLevel::Units => match pow_rational(&b, &e) {
            Some(result) => {
                root.swap(&mut TermBuf::ratio(result).term_mut());
                true
            }
            None => false,
        },
        (Some(b), _) if b.is_one() => {
            root.swap(&mut TermBuf::one().term_mut());
            true
        }
        (_, Some(e)) if e.is_zero() => {
            root.swap(&mut TermBuf::one().term_mut());
            true
        }
        (_, Some(e)) if e.is_one() => {
            let mut arg = root.pop_first_arg().unwrap();
            root.swap(&mut arg.term_mut());
            true
        }
        _ => false,
    }
}

/// `base^exp` when `exp` is a bounded integer; `None` for fractional exponents
/// (left for `sqrt`/other rules), out-of-range exponents, or `0` to a negative
/// power.
fn pow_rational(base: &Rational, exp: &Rational) -> Option<Rational> {
    if !exp.is_integer() {
        return None;
    }
    let e = exp.numer().to_i64()?;
    if e.abs() > MAX_POWER_EXPONENT {
        return None;
    }
    if e == 0 {
        return Some(Rational::one());
    }
    let n = e.unsigned_abs() as u32;
    let numer = base.numer().clone().pow(n);
    let denom = base.denom().clone().pow(n);
    if e > 0 {
        Some(Rational::new(numer, denom))
    } else if numer.is_zero() {
        None
    } else {
        Some(Rational::new(denom, numer))
    }
}

/// `a^(1/2)` as `sqrt(a)`, the one form `sqrt.sym` matches. Above
/// [`NormLevel::Units`], `sqrt(n)^(2k)` as `n^k` for a number `n >= 0`.
fn root_power(base: TermRef, exp: &Rational, level: NormLevel) -> Option<TermBuf> {
    if *exp == Rational::new(1.into(), 2.into()) {
        return Some(TermBuf::symbol("sqrt").arg(base.to_owned()));
    }
    if level <= NormLevel::Units || !base.data().is_symbol_name("sqrt") {
        return None;
    }
    let radicand = base.first_arg()?.data().number().cloned()?;
    if radicand.is_negative() {
        return None;
    }
    let halved = exp / Rational::from(BigInt::from(2));
    Some(TermBuf::ratio(pow_rational(&radicand, &halved)?))
}

pub fn power_argument(root: TermRef) -> TermRef {
    if root.data().is_symbol_name("^") {
        root.first_arg().unwrap()
    } else {
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::symbol::base::calculator_check;

    #[test]
    fn calculator_test() {
        for (source, level_one, level_all) in [
            ("2^1", "2", "2"),
            ("2^0", "1", "1"),
            ("a^1", "a", "a"),
            ("2^3", "2^3", "8"),
            ("(-2)^3", "(-2)^3", "-8"),
            ("(-2)^2", "(-2)^2", "4"),
            ("3^0.5", "sqrt(3)", "sqrt(3)"),
            ("x^0.5", "sqrt(x)", "sqrt(x)"),
            ("sqrt(3)^2", "sqrt(3)^2", "3"),
            ("sqrt(3)^4", "sqrt(3)^4", "9"),
            ("sqrt(-2)^2", "sqrt(-2)^2", "sqrt(-2)^2"),
            ("sqrt(3)^3", "sqrt(3)^3", "sqrt(3)^3"),
            ("sqrt(x)^2", "sqrt(x)^2", "sqrt(x)^2"),
        ] {
            calculator_check(source, source, power, NormLevel::Off);
            calculator_check(source, level_one, power, NormLevel::Units);
            calculator_check(source, level_all, power, NormLevel::Full);
        }
    }
}

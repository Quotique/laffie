use bigdecimal::{BigDecimal as Decimal, One, ToPrimitive, Zero};
use num::traits::Pow;

use crate::{
    term::{FuncSymbol, Subterm, SubtermMut, Symbol, SymbolAttr, SymbolAttrValue, Term},
    NormalizationLevel,
};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("^")
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(100))
        .with_calculator(Box::new(power))
        .build()
}

pub fn power(root: &mut SubtermMut, level: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("^") {
        return false;
    }

    if level == 0.into() {
        return false;
    }

    match (
        root.first_arg().unwrap().data().number(),
        root.last_arg().unwrap().data().number(),
    ) {
        (Some(d1), Some(d2)) if level > 1.into() => {
            if let Some(e) = d2.to_i8() {
                let (m, exp) = d1.as_bigint_and_exponent();
                let result = Decimal::new(m.pow(e.unsigned_abs()), exp * (e.abs() as i64));
                let mut result = Term::number(result);
                while root.pop_first_arg().is_some() {}
                if e >= 0 {
                    root.swap(&mut result.as_subterm_mut());
                } else {
                    *root.data_mut() = Symbol::with_func_symbol("/");
                    root.push_last_arg(Term::one()).push_last_arg(result);
                    root.evaluate(level);
                }
                true
            } else {
                false
            }
        }
        (Some(arg), _) if arg.is_one() => {
            root.swap(&mut Term::one().as_subterm_mut());
            true
        }
        (_, Some(pow)) if pow.is_zero() => {
            root.swap(&mut Term::one().as_subterm_mut());
            true
        }
        (_, Some(pow)) if pow.is_one() => {
            let mut arg = root.pop_first_arg().unwrap();
            root.swap(&mut arg.as_subterm_mut());
            true
        }
        _ => false,
    }
}

pub fn power_argument(root: Subterm) -> Subterm {
    if root.data().is_symbol_name("^") {
        root.first_arg().unwrap()
    } else {
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::func::base::calculator_check;

    #[test]
    fn calculator_test() {
        for (source, level_one, level_all) in [
            ("2^1", "2", "2"),
            ("2^0", "1", "1"),
            ("a^1", "a", "a"),
            ("2^3", "2^3", "8"),
            ("(-2)^3", "(-2)^3", "-8"),
            ("(-2)^2", "(-2)^2", "4"),
        ] {
            calculator_check(source, source, power, NormalizationLevel(0));
            calculator_check(source, level_one, power, NormalizationLevel(1));
            calculator_check(source, level_all, power, NormalizationLevel::max());
        }
    }
}

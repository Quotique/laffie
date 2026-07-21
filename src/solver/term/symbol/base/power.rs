use std::collections::HashMap;

use bigdecimal::{BigDecimal as Decimal, One, ToPrimitive, Zero};
use num::traits::Pow;

use super::SymbolProgram;
use crate::{
    NormLevel,
    term::{Atom, SymbolAttr, SymbolAttrValue, Term, TermBuf, TermMut, TermRef, sym},
};

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

    match (
        root.first_arg().unwrap().data().number(),
        root.last_arg().unwrap().data().number(),
    ) {
        (Some(d1), Some(d2)) if level > NormLevel::Units => {
            if let Some(e) = d2.to_i8() {
                let (m, exp) = d1.as_bigint_and_exponent();
                let result = Decimal::new(m.pow(e.unsigned_abs()), exp * (e.abs() as i64));
                let mut result = TermBuf::number(result);
                while root.pop_first_arg().is_some() {}
                if e >= 0 {
                    root.swap(&mut result.term_mut());
                } else {
                    // Reciprocal left for the normalization fixpoint to fold.
                    *root.data_mut() = Atom::from(sym("/"));
                    root.push_last_arg(TermBuf::one()).push_last_arg(result);
                }
                true
            } else {
                false
            }
        }
        (Some(arg), _) if arg.is_one() => {
            root.swap(&mut TermBuf::one().term_mut());
            true
        }
        (_, Some(pow)) if pow.is_zero() => {
            root.swap(&mut TermBuf::one().term_mut());
            true
        }
        (_, Some(pow)) if pow.is_one() => {
            let mut arg = root.pop_first_arg().unwrap();
            root.swap(&mut arg.term_mut());
            true
        }
        _ => false,
    }
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
        ] {
            calculator_check(source, source, power, NormLevel::Off);
            calculator_check(source, level_one, power, NormLevel::Units);
            calculator_check(source, level_all, power, NormLevel::Full);
        }
    }
}

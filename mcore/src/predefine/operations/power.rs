use bigdecimal::{BigDecimal as Decimal, One, ToPrimitive, Zero};
use num::traits::Pow;
use trees::tr;

use crate::{
    predefine::symbol_by_name,
    term::{swap_node, FuncSymbol, NodeMapping, Symbol, SymbolAttr, SymbolAttrValue, TermNode},
    NormalizationLevel,
};

use super::to_const;

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("^")
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(100))
        .with_calculator(Box::new(power))
        .build()
}

pub fn power(root: &mut TermNode, level: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("^") {
        return false;
    }

    if level == 0.into() {
        return false;
    }

    match (
        to_const(root.front().unwrap()),
        to_const(root.back().unwrap()),
    ) {
        (Some(d1), Some(d2)) if level > 1.into() => {
            if let Some(e) = d2.to_i8() {
                let (m, exp) = d1.as_bigint_and_exponent();
                let result = Decimal::new(m.pow(e.unsigned_abs()), exp * (e.abs() as i64));
                let mut result = if result > Decimal::zero() {
                    tr(Symbol::Number(result))
                } else {
                    tr(Symbol::with_func_symbol("-")) / tr(Symbol::Number(-result))
                };
                // TODO: negative result
                while root.pop_front().is_some() {}
                if e >= 0 {
                    swap_node(root, &mut result.root_mut());
                } else {
                    *root.data_mut() = Symbol::FuncSymbol(symbol_by_name("/").unwrap());
                    root.push_back(tr(Symbol::Number(Decimal::one())));
                    root.push_back(result);
                    root.evaluate(level);
                }
                true
            } else {
                false
            }
        }
        (Some(arg), _) if arg.is_one() => {
            swap_node(root, &mut tr(Symbol::Number(1.into())).root_mut());
            true
        }
        (_, Some(pow)) if pow.is_zero() => {
            swap_node(root, &mut tr(Symbol::Number(1.into())).root_mut());
            true
        }
        (_, Some(pow)) if pow.is_one() => {
            let mut arg = root.pop_front().unwrap();
            swap_node(root, &mut arg.root_mut());
            true
        }
        _ => false,
    }
}

pub fn power_argument(root: &TermNode) -> &TermNode {
    if root.data().is_symbol_name("^") {
        root.front().unwrap()
    } else {
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predefine::operations::calculator_check;

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

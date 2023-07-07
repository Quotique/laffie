use bigdecimal::{BigDecimal as Decimal, One, ToPrimitive, Zero};
use num::traits::Pow;
use trees::tr;

use crate::{
    predefine::symbol_by_name,
    statement::{
        symbols::{Symbol, SymbolAttr, SymbolAttrValue},
        term::{StatementNode, Term},
        tree_utils::{swap_node, NodeMapping},
    },
    NormalizationLevel,
};

use super::to_const;

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("^")
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(100))
        .with_calculator(Box::new(power))
        .build()
        .unwrap()
}

pub fn power(root: &mut StatementNode, level: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("^") {
        return false;
    }

    if level == 0.into() {
        return false;
    }

    match (
        to_const(&root.front().unwrap()),
        to_const(&root.back().unwrap()),
    ) {
        (Some(d1), Some(d2)) if level > 1.into() => {
            if let Some(e) = d2.to_i8() {
                let (m, exp) = d1.as_bigint_and_exponent();
                let result = Decimal::new(m.pow(e.unsigned_abs()), exp * (e.abs() as i64));
                let mut result = if result > Decimal::zero() {
                    tr(Term::Number(result))
                } else {
                    tr(Term::with_symbol_name("-").unwrap()) / tr(Term::Number(-result))
                };
                // TODO: negative result
                while root.pop_front().is_some() {}
                if e >= 0 {
                    swap_node(root, &mut result.root_mut());
                } else {
                    *root.data_mut() = Term::Symbol(symbol_by_name("/").unwrap().id);
                    root.push_back(tr(Term::Number(Decimal::one())));
                    root.push_back(result);
                    root.evaluate(level);
                }
                true
            } else {
                false
            }
        }
        (_, Some(pow)) if pow.is_zero() => {
            swap_node(root, &mut tr(Term::Number(1.into())).root_mut());
            return true;
        }
        (_, Some(pow)) if pow.is_one() => {
            let mut arg = root.pop_front().unwrap();
            swap_node(root, &mut arg.root_mut());
            true
        }
        _ => false,
    }
}

pub fn power_argument(root: &StatementNode) -> &StatementNode {
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

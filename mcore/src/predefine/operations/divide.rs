use std::cmp::max;

use bigdecimal::{BigDecimal as Decimal, One, Zero};
use num::integer::gcd;
use num_bigint::{BigInt, ToBigInt};
use trees::tr;

use crate::statement::{
    symbols::{Symbol, SymbolAttr, SymbolAttrValue},
    term::{StatementNode, Term},
    tree_utils::swap_node,
};

use super::{MAX_DEC_CONVERSION_EXP, MAX_DEC_CONVERSION_VALUE};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("/")
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(200))
        .with_calculator(Box::new(divide))
        .build()
        .unwrap()
}

pub fn divide(root: &mut StatementNode) -> bool {
    if !root.data().is_symbol_name("/") {
        return false;
    }

    let mut result = false;

    match (&root.front().unwrap().data(), &root.back().unwrap().data()) {
        (Term::Number(d1), Term::Number(d2)) => {
            let (num_m, num_e) = d1.clone().into_bigint_and_exponent();
            let (den_m, den_e) = d2.clone().into_bigint_and_exponent();

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
            let (num_m, den_m) = (num_m / g.clone(), den_m / g);

            result = true;
            root.pop_back();
            root.pop_back();

            if den_m.is_one() {
                *root.data_mut() = Term::Number(Decimal::from(num_m));
            } else if (BigInt::from(MAX_DEC_CONVERSION_VALUE) % den_m.clone()).is_zero() {
                *root.data_mut() = Term::Number(
                    Decimal::new(
                        num_m * (BigInt::from(MAX_DEC_CONVERSION_VALUE) / den_m),
                        MAX_DEC_CONVERSION_EXP,
                    )
                    .normalized(),
                );
            } else {
                root.push_back(tr(Term::Number(Decimal::from(num_m))));
                root.push_back(tr(Term::Number(Decimal::from(den_m))));
            }
        }
        (_, Term::Number(d)) => {
            if d.is_one() {
                let mut child = root.pop_front().unwrap();
                swap_node(root, &mut child.root_mut());
            }
        }
        (_, _) => {}
    }

    result
}

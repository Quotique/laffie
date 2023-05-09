use bigdecimal::{BigDecimal as Decimal, One, ToPrimitive};
use num::traits::Pow;
use trees::tr;

use crate::{
    predefine::symbol_by_name,
    statement::{
        symbols::Symbol,
        term::{StatementNode, Term},
        tree_utils::NodeMapping,
    },
    NormalizationLevel,
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("^")
        .with_calculator(Box::new(power))
        .build()
        .unwrap()
}

pub fn power(root: &mut StatementNode, level: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("^") {
        return false;
    }

    let mut result = false;

    if let (Term::Number(d1), Term::Number(d2)) =
        (&root.front().unwrap().data(), &root.back().unwrap().data())
    {
        if let Some(e) = d2.to_i8() {
            result = true;
            let (m, exp) = d1.as_bigint_and_exponent();
            let result = Decimal::new(m.pow(e.unsigned_abs()), exp * (e.abs() as i64));
            while root.pop_front().is_some() {}
            if e >= 0 {
                *root.data_mut() = Term::Number(result);
            } else {
                *root.data_mut() = Term::Symbol(symbol_by_name("/").unwrap().id);
                root.push_back(tr(Term::Number(Decimal::one())));
                root.push_back(tr(Term::Number(result)));
                root.evaluate(level);
            }
        }
    }

    result
}

pub fn power_argument(root: &StatementNode) -> &StatementNode {
    if root.data().is_symbol_name("^") {
        root.front().unwrap()
    } else {
        root
    }
}

use bigdecimal::BigDecimal as Decimal;
use num::Integer;

use crate::statement::{
    symbols::Symbol,
    term::{StatementNode, Term},
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("sqrt")
        .with_calculator(Box::new(sqrt))
        .build()
        .unwrap()
}

pub fn sqrt(root: &mut StatementNode) -> bool {
    if !root.data().is_symbol_name("sqrt") {
        return false;
    }

    if root.degree() != 1 {
        panic!("'sqrt' is unary operator!");
    }

    let last = root.pop_back().unwrap();
    if let Term::Number(d) = &last.data() {
        if d >= &Decimal::from(0) {
            let (mut m, mut e) = d.as_bigint_and_exponent();
            if e.is_odd() {
                m *= 10;
                e -= 1;
            }
            let r = m.sqrt();
            if m == &r * &r {
                *root.data_mut() = Term::Number(Decimal::new(r, e / 2));
                return true;
            }
        }
    }
    root.push_back(last);

    false
}

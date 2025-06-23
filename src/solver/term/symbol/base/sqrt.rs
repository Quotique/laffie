use bigdecimal::BigDecimal as Decimal;
use num::Integer;

use super::SymbolProgram;
use crate::{
    term::{SubtermMut, TermNode},
    NormalizationLevel,
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "sqrt".into(),
        calculator: Box::new(sqrt),
        ..Default::default()
    }
}

pub fn sqrt(root: &mut SubtermMut, _: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("sqrt") {
        return false;
    }

    if root.degree() != 1 {
        panic!("'sqrt' is unary operator!");
    }

    let last = root.pop_last_arg().unwrap();
    if let TermNode::Number(d) = &last.data() {
        if d >= &Decimal::from(0) {
            let (mut m, mut e) = d.as_bigint_and_exponent();
            if e.is_odd() {
                m *= 10;
                e -= 1;
            }
            let r = m.sqrt();
            if m == &r * &r {
                *root.data_mut() = TermNode::Number(Decimal::new(r, e / 2));
                return true;
            }
        }
    }
    root.push_last_arg(last);

    false
}

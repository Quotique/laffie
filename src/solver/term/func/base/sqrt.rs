use bigdecimal::BigDecimal as Decimal;
use num::Integer;

use crate::{
    term::{FuncSymbol, Symbol, SymbolNodeMut},
    NormalizationLevel,
};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("sqrt")
        .with_calculator(Box::new(sqrt))
        .build()
}

pub fn sqrt(root: &mut SymbolNodeMut, _: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("sqrt") {
        return false;
    }

    if root.degree() != 1 {
        panic!("'sqrt' is unary operator!");
    }

    let last = root.pop_back().unwrap();
    if let Symbol::Number(d) = &last.data() {
        if d >= &Decimal::from(0) {
            let (mut m, mut e) = d.as_bigint_and_exponent();
            if e.is_odd() {
                m *= 10;
                e -= 1;
            }
            let r = m.sqrt();
            if m == &r * &r {
                *root.data_mut() = Symbol::Number(Decimal::new(r, e / 2));
                return true;
            }
        }
    }
    root.push_back(last);

    false
}

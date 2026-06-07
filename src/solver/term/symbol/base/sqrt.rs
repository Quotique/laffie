use bigdecimal::BigDecimal as Decimal;
use num::{BigInt, Integer, One, Signed, Zero};

use super::{
    SymbolProgram,
    mul::{extract_numeric_factor, prepend_factor},
};
use crate::{
    NormalizationLevel,
    term::{Atom, TermBuf, TermMut, TermRef},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "sqrt".into(),
        calculator: Box::new(sqrt),
        ..Default::default()
    }
}

pub fn sqrt(root: &mut TermMut, _: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("sqrt") {
        return false;
    }

    if root.degree() != 1 {
        panic!("'sqrt' is unary operator!");
    }

    let last = root.pop_last_arg().unwrap();
    if let Atom::Number(d) = &last.data() &&
        d >= &Decimal::from(0) &&
        let Some(simplified) = simplify_sqrt_number(d)
    {
        *root.data_mut() = Atom::Number(simplified);
        return true;
    }

    if let Some((p, new_arg)) = factor_sqrt_product(last.term()) {
        let sqrt_term = TermBuf::symbol("sqrt").arg(new_arg);
        let mut product = TermBuf::symbol("*").arg(TermBuf::number(p)).arg(sqrt_term);
        product = product.normalize(NormalizationLevel::max());
        root.swap(&mut product.term_mut());
        return true;
    }

    root.push_last_arg(last);

    false
}

/// Returns √n as a Decimal if n is a perfect square.
fn simplify_sqrt_number(d: &Decimal) -> Option<Decimal> {
    let (mut m, mut e) = d.as_bigint_and_exponent();
    if e.is_odd() {
        m *= 10;
        e += 1;
    }
    let r = m.sqrt();
    if m == &r * &r {
        Some(Decimal::new(r, e / 2).normalized())
    } else {
        None
    }
}

/// Pulls the largest square factor `p` out of a `*`-rooted sqrt argument.
/// The sign of the original numeric child stays inside the returned rest.
fn factor_sqrt_product(node: TermRef) -> Option<(Decimal, TermBuf)> {
    let (factor, rest) = extract_numeric_factor(node)?;
    let (mut m, mut e) = factor.as_bigint_and_exponent();
    if m.is_zero() {
        return None;
    }
    if e.is_odd() {
        m *= 10;
        e += 1;
    }
    let q = square_free_part(m.clone());
    if q == m {
        return None;
    }
    let p = (&m / &q).sqrt();

    let new_arg = if q.is_one() {
        rest
    } else {
        prepend_factor(TermBuf::number(q), rest)
    };
    Some((Decimal::new(p, e / 2).normalized(), new_arg))
}

/// Square-free part of `n`, sign-preserving. Caller recovers the extracted
/// root via `(n / q).sqrt()`.
fn square_free_part(mut n: BigInt) -> BigInt {
    let bound = n.abs().sqrt();
    let mut k = BigInt::from(2);
    while k <= bound {
        let sq = &k * &k;
        while (&n % &sq).is_zero() {
            n /= &sq;
        }
        k += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NormalizationLevel, term::term_with_vars};

    #[test]
    fn sqrt_of_perfect_square_number() {
        let mut t = term_with_vars("sqrt(4)");
        t.term_mut().normalize(NormalizationLevel::max());
        assert_eq!(t.term().to_string(), "2");
    }

    #[test]
    fn sqrt_factors_perfect_square_out_of_product() {
        let mut t = term_with_vars("sqrt(4*a)");
        t.term_mut().normalize(NormalizationLevel::max());
        assert_eq!(t.term().to_string(), "2*sqrt(a)");
    }

    #[test]
    fn sqrt_factors_partial_square() {
        let mut t = term_with_vars("sqrt(12*a)");
        t.term_mut().normalize(NormalizationLevel::max());
        assert_eq!(t.term().to_string(), "2*sqrt(3*a)");
    }

    #[test]
    fn sqrt_factors_partial_square_multi() {
        let mut t = term_with_vars("sqrt(48*(-a))");
        t.term_mut().normalize(NormalizationLevel::max());
        assert_eq!(t.term().to_string(), "4*sqrt(-3*a)");
    }

    #[test]
    fn sqrt_of_perfect_square_with_negative_scale() {
        // (10, -3): odd exponent — parity fix should preserve the value.
        let mut t =
            TermBuf::symbol("sqrt").arg(TermBuf::number(Decimal::new(BigInt::from(10), -3)));
        t.term_mut().normalize(NormalizationLevel::max());
        assert_eq!(t.term().to_string(), "100");
    }

    #[test]
    fn sqrt_factors_with_even_nonzero_scale() {
        // (4800, 2) == 48: even nonzero exponent should still factor.
        let arg = TermBuf::symbol("*")
            .arg(TermBuf::number(Decimal::new(BigInt::from(4800), 2)))
            .arg(TermBuf::variable("a"));
        let mut t = TermBuf::symbol("sqrt").arg(arg);
        t.term_mut().normalize(NormalizationLevel::max());
        // sqrt(48a) = 4·sqrt(3a)
        assert_eq!(t.term().to_string(), "4*sqrt(3*a)");
    }

    #[test]
    fn sqrt_no_op_on_unfactorable() {
        let mut t = term_with_vars("sqrt(a)");
        t.term_mut().normalize(NormalizationLevel::max());
        assert_eq!(t.term().to_string(), "sqrt(a)");
    }

    #[test]
    fn square_free_part_basic() {
        assert_eq!(square_free_part(BigInt::from(4)), BigInt::from(1));
        assert_eq!(square_free_part(BigInt::from(12)), BigInt::from(3));
        assert_eq!(square_free_part(BigInt::from(72)), BigInt::from(2));
        assert_eq!(square_free_part(BigInt::from(7)), BigInt::from(7));
    }

    #[test]
    fn square_free_part_negative() {
        assert_eq!(square_free_part(BigInt::from(-4)), BigInt::from(-1));
        assert_eq!(square_free_part(BigInt::from(-48)), BigInt::from(-3));
        assert_eq!(square_free_part(BigInt::from(-7)), BigInt::from(-7));
    }
}

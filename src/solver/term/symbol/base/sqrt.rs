use num::{BigInt, One, Signed, Zero};

use super::{
    SymbolProgram,
    mul::{extract_numeric_factor, prepend_factor},
};
use crate::{
    NormLevel, Rational,
    term::{Atom, TermBuf, TermMut, TermRef},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "sqrt".into(),
        calculator: Box::new(sqrt),
        ..Default::default()
    }
}

pub fn sqrt(root: &mut TermMut, _: NormLevel) -> bool {
    if !root.data().is_symbol_name("sqrt") {
        return false;
    }

    if root.degree() != 1 {
        panic!("'sqrt' is unary operator!");
    }

    let last = root.pop_last_arg().unwrap();
    if let Atom::Number(d) = &last.data() &&
        !d.is_negative() &&
        let Some(simplified) = simplify_sqrt_number(d)
    {
        *root.data_mut() = Atom::Number(simplified);
        return true;
    }

    if let Some((p, new_arg)) = factor_sqrt_product(last.term()) {
        let sqrt_term = TermBuf::symbol("sqrt").arg(new_arg);
        // Left for the normalization fixpoint to finish.
        let mut product = TermBuf::symbol("*").arg(TermBuf::ratio(p)).arg(sqrt_term);
        root.swap(&mut product.term_mut());
        return true;
    }

    root.push_last_arg(last);

    false
}

/// Returns √r as a rational if both numerator and denominator are perfect
/// squares. `None` for a negative `r`; the denominator is always positive.
fn simplify_sqrt_number(r: &Rational) -> Option<Rational> {
    if r.numer().is_negative() {
        return None;
    }
    // `n.sqrt()` is the floor root; a perfect square satisfies `root^2 == n`.
    let exact = |n: &BigInt| {
        let root = n.sqrt();
        (&root * &root == *n).then_some(root)
    };
    Some(Rational::new(exact(r.numer())?, exact(r.denom())?))
}

/// Pulls the largest square factor `p` out of a `*`-rooted sqrt argument.
/// The sign of the original numeric child stays inside the returned rest.
fn factor_sqrt_product(node: TermRef) -> Option<(Rational, TermBuf)> {
    let (c, rest) = extract_numeric_factor(node)?;
    if c.is_zero() {
        return None;
    }
    let (p_num, q_num) = pull_square(c.numer().abs());
    let (p_den, q_den) = pull_square(c.denom().clone());
    if p_num.is_one() && p_den.is_one() {
        return None;
    }
    let extracted = Rational::new(p_num, p_den);
    let inner_numer = if c.is_negative() { -q_num } else { q_num };
    let inner_coeff = Rational::new(inner_numer, q_den);

    let new_arg = if inner_coeff.is_one() {
        rest
    } else {
        prepend_factor(TermBuf::ratio(inner_coeff), rest)
    };
    Some((extracted, new_arg))
}

/// Splits non-negative `n` into `(root, square_free)` with
/// `n == root^2 * square_free`.
fn pull_square(n: BigInt) -> (BigInt, BigInt) {
    if n.is_zero() {
        return (BigInt::zero(), BigInt::zero());
    }
    let mut q = n.clone();
    let bound = q.sqrt();
    let mut k = BigInt::from(2);
    while k <= bound {
        let sq = &k * &k;
        while (&q % &sq).is_zero() {
            q /= &sq;
        }
        k += 1;
    }
    let root = (&n / &q).sqrt();
    (root, q)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NormLevel, term::term_with_vars};

    #[test]
    fn sqrt_of_perfect_square_number() {
        let mut t = term_with_vars("sqrt(4)");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "2");
    }

    #[test]
    fn sqrt_of_perfect_square_fraction() {
        // sqrt(1/4) = 1/2 -> "0.5"
        let t = TermBuf::symbol("sqrt").arg(TermBuf::ratio(Rational::new(1.into(), 4.into())));
        let mut t = t;
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "0.5");
    }

    #[test]
    fn sqrt_factors_perfect_square_out_of_product() {
        let mut t = term_with_vars("sqrt(4*a)");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "2*sqrt(a)");
    }

    #[test]
    fn sqrt_factors_partial_square() {
        let mut t = term_with_vars("sqrt(12*a)");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "2*sqrt(3*a)");
    }

    #[test]
    fn sqrt_factors_partial_square_multi() {
        let mut t = term_with_vars("sqrt(48*(-a))");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "4*sqrt(-3*a)");
    }

    #[test]
    fn sqrt_of_perfect_square_large() {
        let mut t = term_with_vars("sqrt(10000)");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "100");
    }

    #[test]
    fn sqrt_factors_large_coefficient() {
        let mut t = term_with_vars("sqrt(48*a)");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "4*sqrt(3*a)");
    }

    #[test]
    fn sqrt_no_op_on_unfactorable() {
        let mut t = term_with_vars("sqrt(a)");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "sqrt(a)");
    }

    #[test]
    fn sqrt_factors_fraction_coefficient() {
        // sqrt((9/4)*a) = (3/2)*sqrt(a) -> square factors pulled from both the
        // numerator and the denominator of the coefficient.
        let mut t = term_with_vars("sqrt(2.25*a)");
        t.term_mut().normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "1.5*sqrt(a)");
    }

    #[test]
    fn pull_square_splits() {
        // n == root^2 * square_free (non-negative input).
        assert_eq!(
            pull_square(BigInt::from(4)),
            (BigInt::from(2), BigInt::from(1))
        );
        assert_eq!(
            pull_square(BigInt::from(12)),
            (BigInt::from(2), BigInt::from(3))
        );
        assert_eq!(
            pull_square(BigInt::from(48)),
            (BigInt::from(4), BigInt::from(3))
        );
        assert_eq!(
            pull_square(BigInt::from(72)),
            (BigInt::from(6), BigInt::from(2))
        );
        assert_eq!(
            pull_square(BigInt::from(7)),
            (BigInt::from(1), BigInt::from(7))
        );
    }
}

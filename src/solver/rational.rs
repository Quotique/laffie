use std::str::FromStr;

use num::{BigInt, One, Signed, Zero};

use crate::Rational;

/// Parse a decimal literal (`"2.5"`, `"-3"`, `".25"`) into an exact rational.
pub fn from_decimal_str(s: &str) -> Option<Rational> {
    let s = s.trim();
    match s.split_once('.') {
        Some((int, frac)) => {
            let numer = BigInt::from_str(&format!("{int}{frac}")).ok()?;
            let denom = BigInt::from(10).pow(frac.len() as u32);
            Some(Rational::new(numer, denom))
        }
        None => Some(Rational::from_integer(BigInt::from_str(s).ok()?)),
    }
}

/// Human-facing rendering of a rational: a bare integer when the denominator is
/// 1, a terminating decimal when the denominator is 2·5-smooth, otherwise
/// `numer/denom`. The sign is included; callers wrap negatives as needed.
pub fn number_to_string(r: &Rational) -> String {
    if r.denom().is_one() {
        return r.numer().to_string();
    }
    to_decimal(r).unwrap_or_else(|| format!("{}/{}", r.numer(), r.denom()))
}

/// Decimal rendering when the (reduced, positive) denominator is `2^a * 5^b`;
/// `None` otherwise.
fn to_decimal(r: &Rational) -> Option<String> {
    let denom = r.denom();
    let (twos, rem) = factor_out(denom.clone(), 2);
    let (fives, rem) = factor_out(rem, 5);
    if !rem.is_one() {
        return None;
    }

    let k = twos.max(fives);
    let ten_k = BigInt::from(10).pow(k);
    let scaled = r.numer() * (&ten_k / denom);
    let neg = scaled.is_negative();
    let mag = scaled.abs();

    let int_part = &mag / &ten_k;
    let frac_digits = (&mag % &ten_k).to_string();
    let pad = "0".repeat((k as usize).saturating_sub(frac_digits.len()));
    let sign = if neg { "-" } else { "" };
    Some(format!("{sign}{int_part}.{pad}{frac_digits}"))
}

/// Serde codec for a [`Rational`] as the compact string `numer/denom` (or a
/// bare integer), used via `#[serde(with = "...")]` on `Atom::Number`. Keeps
/// the persisted form readable and independent of `num`'s internal layout.
pub mod serde_str {
    use std::str::FromStr;

    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    use crate::Rational;

    pub fn serialize<S: Serializer>(r: &Rational, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&r.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Rational, D::Error> {
        let s = String::deserialize(d)?;
        Rational::from_str(&s).map_err(Error::custom)
    }
}

/// Returns how many times `p` divides `n`, and the remaining cofactor.
fn factor_out(mut n: BigInt, p: u32) -> (u32, BigInt) {
    let p = BigInt::from(p);
    let mut count = 0;
    while (&n % &p).is_zero() {
        n /= &p;
        count += 1;
    }
    (count, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> String {
        number_to_string(&from_decimal_str(s).unwrap())
    }

    #[test]
    fn integers_render_bare() {
        assert_eq!(dec("3"), "3");
        assert_eq!(dec("-5"), "-5");
        assert_eq!(dec("0"), "0");
    }

    #[test]
    fn terminating_decimals() {
        assert_eq!(dec("2.5"), "2.5");
        assert_eq!(dec("1.4"), "1.4");
        assert_eq!(dec("-2.5"), "-2.5");
        assert_eq!(dec("0.015625"), "0.015625"); // 1/64
        assert_eq!(dec("0.05"), "0.05");
    }

    #[test]
    fn non_smooth_denominator_is_fraction() {
        // 1/3 and 5/6 have denominators with a factor of 3.
        assert_eq!(number_to_string(&Rational::new(1.into(), 3.into())), "1/3");
        assert_eq!(number_to_string(&Rational::new(5.into(), 6.into())), "5/6");
        assert_eq!(
            number_to_string(&Rational::new((-5).into(), 3.into())),
            "-5/3"
        );
    }

    #[test]
    fn halves_and_quarters() {
        assert_eq!(number_to_string(&Rational::new(1.into(), 2.into())), "0.5");
        assert_eq!(number_to_string(&Rational::new(1.into(), 4.into())), "0.25");
    }

    #[test]
    fn auto_reduces() {
        // 6/4 -> 3/2 -> "1.5"
        assert_eq!(number_to_string(&Rational::new(6.into(), 4.into())), "1.5");
        // 9/2 -> "4.5"
        assert_eq!(number_to_string(&Rational::new(9.into(), 2.into())), "4.5");
    }

    #[test]
    fn from_decimal_reduces() {
        assert_eq!(
            from_decimal_str("1.50"),
            Some(Rational::new(3.into(), 2.into()))
        );
        assert_eq!(
            from_decimal_str("-0.5"),
            Some(Rational::new((-1).into(), 2.into()))
        );
    }
}

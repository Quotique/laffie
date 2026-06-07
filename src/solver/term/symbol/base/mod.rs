pub mod answer;
pub mod divide;
pub mod equal;
pub mod inequal;
pub mod is;
pub mod less;
pub mod less_or_equal;
pub mod more;
pub mod more_or_equal;
pub mod mul;
pub mod op_in;
pub mod op_not;
pub mod op_true;
pub mod plus;
pub mod power;
pub mod sqrt;
pub mod substitute;
pub mod symbolic_eq;

use std::cmp::Ordering;

pub use super::SymbolProgram;
use crate::term::{Term, TermRef};

#[cfg(test)]
use crate::{
    NormalizationLevel,
    term::{TermMut, term_with_params},
};

pub const MAX_DEC_CONVERSION_EXP: i64 = 6;

fn compare_numbers(left: TermRef, right: TermRef) -> Option<Ordering> {
    let left_num = left.data().number()?;
    let right_num = right.data().number()?;

    Some(left_num.cmp(right_num))
}

#[cfg(test)]
pub fn calculator_check(
    src: &'static str,
    res: &'static str,
    f: impl Fn(&mut TermMut, NormalizationLevel) -> bool,
    level: NormalizationLevel,
) {
    let mut s = term_with_params(src);
    let r = term_with_params(res);
    assert_eq!(
        f(&mut s.term_mut(), level),
        src != res,
        "{src} {res} l:{level}"
    );
    assert_eq!(s, r, "{src} {res} l:{level}");
}

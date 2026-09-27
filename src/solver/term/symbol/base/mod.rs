pub mod abs;
pub mod answer;
pub mod denom;
pub mod divide;
pub mod domain;
pub mod equal;
pub mod inequal;
pub mod is;
pub mod less;
pub mod less_or_equal;
pub mod logic_and;
pub mod logic_or;
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
use crate::term::{Term, TermBuf, TermMut, TermRef};

#[cfg(test)]
use crate::{NormLevel, term::term_with_params};

/// Removes duplicate operands of an idempotent connective (`||`, `&&`);
/// collapses to the sole remaining operand.
pub(super) fn dedup_operands(root: &mut TermMut, name: &str) -> bool {
    if !root.data().is_symbol_name(name) || root.degree() < 2 {
        return false;
    }
    let mut seen: Vec<TermBuf> = Vec::new();
    let mut changed = false;
    for mut child in root.iter_mut() {
        let owned = child.to_term();
        if seen.iter().any(|s| s.term() == owned.term()) {
            child.detach();
            changed = true;
        } else {
            seen.push(owned);
        }
    }
    if changed && root.degree() == 1 {
        let mut only = root.pop_first_arg().unwrap();
        root.swap(&mut only.term_mut());
    }
    changed
}

fn compare_numbers(left: TermRef, right: TermRef) -> Option<Ordering> {
    let left_num = left.data().number()?;
    let right_num = right.data().number()?;

    Some(left_num.cmp(right_num))
}

#[cfg(test)]
pub fn calculator_check(
    src: &'static str,
    res: &'static str,
    f: impl Fn(&mut TermMut, NormLevel) -> bool,
    level: NormLevel,
) {
    let mut s = term_with_params(src);
    let r = term_with_params(res);
    assert_eq!(
        f(&mut s.term_mut(), level),
        src != res,
        "{src} {res} l:{level:?}"
    );
    assert_eq!(s, r, "{src} {res} l:{level:?}");
}

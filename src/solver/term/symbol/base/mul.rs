use std::{cmp::Ordering, collections::HashMap};

use bigdecimal::{BigDecimal as Decimal, One, Zero};
use indexmap::IndexMap;

use crate::{
    NormalizationLevel,
    term::{Atom, SymbolAttr, SymbolAttrValue, Term, TermBuf, TermMut, TermRef},
};

use super::{SymbolProgram, plus::plus, power::power_argument};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "*".into(),
        attrs: HashMap::from([
            (SymbolAttr::Associative, SymbolAttrValue::None),
            (SymbolAttr::Commutative, SymbolAttrValue::None),
            (SymbolAttr::Infix, SymbolAttrValue::UInt(200)),
        ]),
        calculator: Box::new(multiply),
        arg_cmp: Box::new(ordering),
        ..Default::default()
    }
}

pub fn multiply(root: &mut TermMut, level: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("*") {
        return false;
    }

    match level {
        NormalizationLevel(0) => false,
        NormalizationLevel(1) => {
            if root
                .iter()
                .any(|x| x.data().is_number_value(&Decimal::zero()))
            {
                root.swap(&mut TermBuf::zero().term_mut());
                return true;
            }
            let result = root.iter_mut().fold(false, |acc, mut x| {
                if x.data().is_number_value(&Decimal::one()) {
                    x.detach();
                    true
                } else {
                    acc
                }
            });
            remove_unused_mul(root) || result
        }
        NormalizationLevel(2) => {
            let (constant, result) = fold_constant(root);
            attach_constant(root, constant) || remove_unused_mul(root) || result
        }
        _ => {
            let (constant, result) = fold_constant(root);
            let (powers, result) = root.iter_mut().fold(
                (IndexMap::<TermBuf, TermBuf>::new(), result),
                |acc, mut x| {
                    let power = extract_power(&mut x);
                    let (mut powers, mut result) = acc;
                    powers
                        .entry(x.detach())
                        .and_modify(|p| {
                            let mut new_pow =
                                TermBuf::symbol("+").arg(p.clone()).arg(power.clone());
                            p.term_mut().swap(&mut new_pow.term_mut());
                            result = true;
                        })
                        .or_insert(power);
                    (powers, result)
                },
            );
            // IndexMap keeps insertion order deterministic; sorting by
            // `ordering` then yields a stable, reproducible factor order (ties
            // between equally-ranked factors fall back to insertion order).
            let powers = {
                let mut powers: Vec<_> = powers.into_iter().collect();
                powers.sort_by(|x, y| ordering(x.0.term(), y.0.term()));
                powers
            };
            for (elem, mut pow) in powers.into_iter() {
                plus(&mut pow.term_mut(), level);
                let arg = merge_power(elem, pow);
                if !arg.data().is_number_value(&Decimal::from(1)) {
                    root.push_last_arg(arg);
                }
            }
            attach_constant(root, constant) || remove_unused_mul(root) || result
        }
    }
}

/// Multiplies `factor` into `rest`, flattening if `rest` is already `*`-rooted.
pub(super) fn prepend_factor(factor: TermBuf, rest: TermBuf) -> TermBuf {
    if rest.term().data().is_symbol_name("*") {
        let mut rest = rest;
        rest.term_mut().push_first_arg(factor);
        rest
    } else {
        TermBuf::symbol("*").arg(factor).arg(rest)
    }
}

/// Folds all numeric children of a `*`-rooted term into one factor and
/// returns it with the remaining product. `None` if no numeric child exists.
pub(super) fn extract_numeric_factor(node: TermRef) -> Option<(Decimal, TermBuf)> {
    if !node.data().is_symbol_name("*") {
        return None;
    }
    let mut owned = node.to_owned();
    let (factor, changed) = fold_constant(&mut owned.term_mut());
    if !changed && factor.is_one() {
        return None;
    }
    remove_unused_mul(&mut owned.term_mut());
    Some((factor, owned))
}

fn attach_constant(root: &mut TermMut, constant: Decimal) -> bool {
    if constant == Decimal::zero() {
        root.swap(&mut TermBuf::zero().term_mut());
        true
    } else if constant != Decimal::one() {
        root.push_first_arg(TermBuf::number(constant));
        false
    } else {
        false
    }
}

fn fold_constant(root: &mut TermMut) -> (Decimal, bool) {
    root.iter_mut()
        .enumerate()
        .fold((Decimal::from(1), false), |acc, (num, mut x)| {
            if let Some(d) = x.data().clone().number() {
                x.detach();
                let res = d.is_one() || num != 0;
                (acc.0 * d, res)
            } else {
                acc
            }
        })
}

fn remove_unused_mul(root: &mut TermMut) -> bool {
    match root.degree() {
        0 => {
            *root.data_mut() = Atom::Number(Decimal::from(1));
            true
        }
        1 => {
            let mut child = root.pop_first_arg().unwrap();
            root.swap(&mut child.term_mut());
            true
        }
        _ => false,
    }
}

fn merge_power(root: TermBuf, pow: TermBuf) -> TermBuf {
    if pow == TermBuf::one() {
        return root;
    }

    if pow == TermBuf::zero() {
        return TermBuf::one();
    }

    TermBuf::symbol("^").arg(root).arg(pow)
}

fn extract_power(root: &mut TermMut) -> TermBuf {
    if root.data().is_symbol_name("^") {
        let power = root.pop_last_arg().unwrap();
        let mut arg = root.pop_first_arg().unwrap();
        assert_eq!(root.degree(), 0);

        root.swap(&mut arg.term_mut());
        power
    } else {
        TermBuf::one()
    }
}

fn ordering(left: TermRef, right: TermRef) -> Ordering {
    // Number < Param < Variable < Symbol < Placeholder
    let pa_left = power_argument(left);
    let pa_right = power_argument(right);

    match (pa_left.data().number(), pa_right.data().number()) {
        (Some(left), Some(right)) => return left.cmp(right),
        (Some(_), _) => return Ordering::Less,
        (_, Some(_)) => return Ordering::Greater,
        _ => {}
    }

    match (pa_left.data(), pa_right.data()) {
        (Atom::Number(left), Atom::Number(right)) => left.cmp(right),
        (Atom::Number(_), _) => Ordering::Less,

        (Atom::Param(left), Atom::Param(right)) => left.cmp(right),
        (Atom::Param(_), Atom::Number(_)) => Ordering::Greater,
        (Atom::Param(_), _) => Ordering::Less,

        (Atom::Variable(left), Atom::Variable(right)) => left.cmp(right),
        (Atom::Variable(_), Atom::Symbol(_)) => Ordering::Less,
        (Atom::Variable(_), Atom::ArgList(_)) => Ordering::Less,
        (Atom::Variable(_), _) => Ordering::Greater,

        (Atom::Symbol(left), Atom::Symbol(right)) => left.cmp(right),
        (Atom::Symbol(_), Atom::ArgList(_)) => Ordering::Less,
        (Atom::Symbol(_), _) => Ordering::Greater,

        (Atom::ArgList(_), Atom::ArgList(_)) => Ordering::Equal,
        (Atom::ArgList(_), _) => Ordering::Greater,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::symbol::base::calculator_check;

    #[test]
    fn calculator_test() {
        for (source, level_one, level_two, level_all) in [
            ("x*y", "x*y", "x*y", "x*y"),
            ("0*x*y", "0", "0", "0"),
            ("1*x*y", "x*y", "x*y", "x*y"),
            ("1*x*y*2", "x*y*2", "2*x*y", "2*x*y"),
            ("1*x*y*2*3", "x*y*2*3", "6*x*y", "6*x*y"),
            ("x*x", "x*x", "x*x", "x^2"),
            ("2*3", "2*3", "6", "6"),
            ("(-2)*3", "(-2)*3", "-6", "-6"),
            ("1*3", "3", "3", "3"),
            ("(-6)*(-x^2)", "(-6)*(-x^2)", "6*x^2", "6*x^2"),
        ] {
            calculator_check(source, source, multiply, NormalizationLevel(0));
            calculator_check(source, level_one, multiply, NormalizationLevel(1));
            calculator_check(source, level_two, multiply, NormalizationLevel(2));
            calculator_check(source, level_all, multiply, NormalizationLevel::max());
        }
    }
}

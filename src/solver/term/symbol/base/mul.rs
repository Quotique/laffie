use std::{cmp::Ordering, collections::HashMap};

use bigdecimal::{BigDecimal as Decimal, One, Zero};

use crate::{
    term::{Subterm, SubtermMut, SymbolAttr, SymbolAttrValue, Term, TermNode},
    NormalizationLevel,
};

use super::{plus::plus, power::power_argument, SymbolProgram};

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

pub fn multiply(root: &mut SubtermMut, level: NormalizationLevel) -> bool {
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
                root.swap(&mut Term::zero().as_subterm_mut());
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
            let (powers, result) =
                root.iter_mut()
                    .fold((HashMap::<Term, Term>::new(), result), |acc, mut x| {
                        let power = extract_power(&mut x);
                        let (mut powers, mut result) = acc;
                        powers
                            .entry(x.detach())
                            .and_modify(|p| {
                                let mut new_pow = Term::symbol("+")
                                    .with_child(p.clone())
                                    .with_child(power.clone());
                                p.as_subterm_mut().swap(&mut new_pow.as_subterm_mut());
                                result = true;
                            })
                            .or_insert(power);
                        (powers, result)
                    });
            #[cfg(test)]
            let powers = {
                let mut powers: Vec<_> = powers.into_iter().collect();
                powers.sort_by(|x, y| ordering(x.0.as_subterm(), y.0.as_subterm()));
                powers
            };
            for (elem, mut pow) in powers.into_iter() {
                plus(&mut pow.as_subterm_mut(), level);
                let arg = merge_power(elem, pow);
                if !arg.data().is_number_value(&Decimal::from(1)) {
                    root.push_last_arg(arg);
                }
            }
            attach_constant(root, constant) || remove_unused_mul(root) || result
        }
    }
}

fn attach_constant(root: &mut SubtermMut, constant: Decimal) -> bool {
    if constant == Decimal::zero() {
        root.swap(&mut Term::zero().as_subterm_mut());
        true
    } else if constant != Decimal::one() {
        root.push_first_arg(Term::number(constant));
        false
    } else {
        false
    }
}

fn fold_constant(root: &mut SubtermMut) -> (Decimal, bool) {
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

fn remove_unused_mul(root: &mut SubtermMut) -> bool {
    match root.degree() {
        0 => {
            *root.data_mut() = TermNode::Number(Decimal::from(1));
            true
        }
        1 => {
            let mut child = root.pop_first_arg().unwrap();
            root.swap(&mut child.as_subterm_mut());
            true
        }
        _ => false,
    }
}

fn merge_power(root: Term, pow: Term) -> Term {
    if pow == Term::one() {
        return root;
    }

    if pow == Term::zero() {
        return Term::one();
    }

    Term::symbol("^").with_child(root).with_child(pow)
}

fn extract_power(root: &mut SubtermMut) -> Term {
    if root.data().is_symbol_name("^") {
        let power = root.pop_last_arg().unwrap();
        let mut arg = root.pop_first_arg().unwrap();
        assert_eq!(root.degree(), 0);

        root.swap(&mut arg.as_subterm_mut());
        power
    } else {
        Term::one()
    }
}

fn ordering(left: Subterm, right: Subterm) -> Ordering {
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
        (TermNode::Number(left), TermNode::Number(right)) => left.cmp(right),
        (TermNode::Number(_), _) => Ordering::Less,

        (TermNode::Param(left), TermNode::Param(right)) => left.cmp(right),
        (TermNode::Param(_), TermNode::Number(_)) => Ordering::Greater,
        (TermNode::Param(_), _) => Ordering::Less,

        (TermNode::Variable(left), TermNode::Variable(right)) => left.cmp(right),
        (TermNode::Variable(_), TermNode::Symbol(_)) => Ordering::Less,
        (TermNode::Variable(_), TermNode::ArgList(_)) => Ordering::Less,
        (TermNode::Variable(_), _) => Ordering::Greater,

        (TermNode::Symbol(left), TermNode::Symbol(right)) => left.cmp(right),
        (TermNode::Symbol(_), TermNode::ArgList(_)) => Ordering::Less,
        (TermNode::Symbol(_), _) => Ordering::Greater,

        (TermNode::ArgList(_), TermNode::ArgList(_)) => Ordering::Equal,
        (TermNode::ArgList(_), _) => Ordering::Greater,
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

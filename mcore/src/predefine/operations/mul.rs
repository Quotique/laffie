use std::{cmp::Ordering, collections::HashMap};

use bigdecimal::{BigDecimal as Decimal, One, Zero};
use trees::{tr, Tree};

use crate::{
    term::{
        func_symbol::{FuncSymbol, SymbolAttr, SymbolAttrValue},
        symbol::Symbol,
        tree_utils::swap_node,
        TermNode,
    },
    NormalizationLevel,
};

use super::{plus::plus, power::power_argument, to_const};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("*")
        .with_attr(SymbolAttr::Associative, SymbolAttrValue::None)
        .with_attr(SymbolAttr::Commutative, SymbolAttrValue::None)
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(200))
        .with_calculator(Box::new(multiply))
        .with_ordering(Box::new(ordering))
        .build()
}

pub fn multiply(root: &mut TermNode, level: NormalizationLevel) -> bool {
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
                swap_node(root, &mut tr(Symbol::Number(0.into())).root_mut());
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
                (HashMap::<Tree<Symbol>, Tree<Symbol>>::new(), result),
                |acc, mut x| {
                    let power = extract_power(&mut x);
                    let (mut powers, mut result) = acc;
                    powers
                        .entry(x.detach())
                        .and_modify(|p| {
                            let mut new_pow =
                                tr(Symbol::with_func_symbol("+")) / p.clone() / power.clone();
                            swap_node(&mut p.root_mut(), &mut new_pow.root_mut());
                            result = true;
                        })
                        .or_insert(power);
                    (powers, result)
                },
            );
            #[cfg(test)]
            let powers = {
                let mut powers: Vec<_> = powers.into_iter().collect();
                powers.sort_by(|x, y| ordering(x.0.root(), y.0.root()));
                powers
            };
            for (elem, mut pow) in powers.into_iter() {
                plus(&mut pow.root_mut(), level);
                let arg = merge_power(elem, pow);
                if !arg.data().is_number_value(&Decimal::from(1)) {
                    root.push_back(arg);
                }
            }
            attach_constant(root, constant) || remove_unused_mul(root) || result
        }
    }
}

fn attach_constant(root: &mut TermNode, constant: Decimal) -> bool {
    if constant == Decimal::zero() {
        swap_node(root, &mut tr(Symbol::Number(0.into())).root_mut());
        true
    } else if constant < Decimal::zero() {
        root.push_front(tr(Symbol::with_func_symbol("-")) / tr(Symbol::Number(-constant)));
        false
    } else if constant != Decimal::one() {
        root.push_front(tr(Symbol::Number(constant)));
        false
    } else {
        false
    }
}

fn fold_constant(root: &mut TermNode) -> (Decimal, bool) {
    root.iter_mut()
        .enumerate()
        .fold((Decimal::from(1), false), |acc, (num, mut x)| {
            if let Some(d) = to_const(&x) {
                x.detach();
                let res = d.is_one() || num != 0;
                (acc.0 * d, res)
            } else if x.data().is_symbol_name("-") {
                let mut front = x.pop_front().unwrap();
                swap_node(&mut x, &mut front.root_mut());
                let res = acc.1 || num != 0;
                (-acc.0, res)
            } else {
                acc
            }
        })
}

fn remove_unused_mul(root: &mut TermNode) -> bool {
    match root.degree() {
        0 => {
            *root.data_mut() = Symbol::Number(Decimal::from(1));
            true
        }
        1 => {
            let mut child = root.pop_front().unwrap();
            swap_node(root, &mut child.root_mut());
            true
        }
        _ => false,
    }
}

fn merge_power(root: Tree<Symbol>, pow: Tree<Symbol>) -> Tree<Symbol> {
    if pow == tr(Symbol::Number(Decimal::from(1))) {
        return root;
    }

    if pow == tr(Symbol::Number(Decimal::from(0))) {
        return tr(Symbol::Number(Decimal::from(1)));
    }

    tr(Symbol::with_func_symbol("^")) / root / pow
}

fn extract_power(root: &mut TermNode) -> Tree<Symbol> {
    if root.data().is_symbol_name("^") {
        let power = root.pop_back().unwrap();
        let mut arg = root.pop_front().unwrap();
        assert_eq!(root.degree(), 0);

        swap_node(root, &mut arg.root_mut());
        power
    } else {
        tr(Symbol::Number(Decimal::from(1)))
    }
}

fn ordering(left: &TermNode, right: &TermNode) -> Ordering {
    // Number < Param < Variable < Symbol < Placeholder
    let pa_left = power_argument(left);
    let pa_right = power_argument(right);

    match (to_const(pa_left), to_const(pa_right)) {
        (Some(left), Some(right)) => return left.cmp(&right),
        (Some(_), _) => return Ordering::Less,
        (_, Some(_)) => return Ordering::Greater,
        _ => {}
    }

    match (pa_left.data(), pa_right.data()) {
        (Symbol::Number(left), Symbol::Number(right)) => left.cmp(right),
        (Symbol::Number(_), _) => Ordering::Less,

        (Symbol::Param(left), Symbol::Param(right)) => left.cmp(right),
        (Symbol::Param(_), Symbol::Number(_)) => Ordering::Greater,
        (Symbol::Param(_), _) => Ordering::Less,

        (Symbol::Variable(left), Symbol::Variable(right)) => left.cmp(right),
        (Symbol::Variable(_), Symbol::FuncSymbol(_)) => Ordering::Less,
        (Symbol::Variable(_), Symbol::Placeholder(_)) => Ordering::Less,
        (Symbol::Variable(_), _) => Ordering::Greater,

        (Symbol::FuncSymbol(left), Symbol::FuncSymbol(right)) => left.cmp(right),
        (Symbol::FuncSymbol(_), Symbol::Placeholder(_)) => Ordering::Less,
        (Symbol::FuncSymbol(_), _) => Ordering::Greater,

        (Symbol::Placeholder(_), Symbol::Placeholder(_)) => Ordering::Equal,
        (Symbol::Placeholder(_), _) => Ordering::Greater,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predefine::operations::calculator_check;

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

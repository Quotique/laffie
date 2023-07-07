use bigdecimal::Zero;
use trees::tr;

use crate::{
    statement::{
        symbols::{Symbol, SymbolAttr, SymbolAttrValue},
        term::{StatementNode, Term},
        tree_utils::{swap_node, NodeMapping},
    },
    NormalizationLevel,
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("-")
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(300))
        .with_calculator(Box::new(minus))
        .build()
        .unwrap()
}

pub fn minus(root: &mut StatementNode, level: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("-") {
        return false;
    }

    match level {
        NormalizationLevel(0) => false,
        NormalizationLevel(1) => remove_zeroes(root),
        _ => {
            if root.degree() == 2 {
                let second = root.pop_back().unwrap();
                *root.data_mut() = Term::with_symbol_name("+").unwrap();
                root.push_back(tr(Term::with_symbol_name("-").unwrap()) / second);
                root.evaluate(level)
            } else if root.front().unwrap().data().is_symbol_name("*") {
                let mut child = root.pop_front().unwrap();
                swap_node(root, &mut child.root_mut());
                let first_arg = root.pop_front().unwrap();
                root.push_front(tr(Term::with_symbol_name("-").unwrap()) / first_arg);
                root.evaluate(level)
            } else {
                remove_zeroes(root)
            }
        }
    }
}

fn remove_zeroes(root: &mut StatementNode) -> bool {
    match root.degree() {
        1 => {
            if let Term::Number(d) = root.back().unwrap().data() {
                if d.is_zero() {
                    swap_node(root, &mut tr(Term::Number(0.into())).root_mut());
                    return true;
                }
            }
            false
        }
        2 => match (&root.front().unwrap().data(), &root.back().unwrap().data()) {
            (Term::Number(d1), Term::Number(d2)) if d1.is_zero() && d2.is_zero() => {
                swap_node(root, &mut tr(Term::Number(0.into())).root_mut());
                true
            }
            (Term::Number(d), _) if d.is_zero() => {
                let _ = root.pop_front().unwrap();
                true
            }
            (_, Term::Number(d)) if d.is_zero() => {
                let mut first = root.pop_front().unwrap();
                swap_node(root, &mut first.root_mut());
                true
            }
            _ => false,
        },
        n => {
            panic!("'-' is binary operator! {} {}", n, root);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predefine::operations::calculator_check;

    #[test]
    fn calculator_test() {
        for (source, level_one, level_two, level_all) in
            [("-3", "-3", "-3", "-3"), ("-0", "0", "0", "0")]
        {
            calculator_check(source, source, minus, NormalizationLevel(0));
            calculator_check(source, level_one, minus, NormalizationLevel(1));
            calculator_check(source, level_two, minus, NormalizationLevel(2));
            calculator_check(source, level_all, minus, NormalizationLevel::max());
        }
    }
}

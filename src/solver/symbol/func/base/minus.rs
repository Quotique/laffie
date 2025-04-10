use bigdecimal::Zero;
use trees::tr;

use crate::{
    symbol::{FuncSymbol, Symbol, SymbolAttr, SymbolAttrValue, SymbolNode},
    term::{swap_node, NodeMapping},
    NormalizationLevel,
};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("-")
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(300))
        .with_calculator(Box::new(minus))
        .build()
}

pub fn minus(root: &mut SymbolNode, level: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("-") {
        return false;
    }

    match level {
        NormalizationLevel(0) => false,
        NormalizationLevel(1) => remove_zeroes(root),
        _ => {
            if root.degree() == 2 {
                let second = root.pop_back().unwrap();
                *root.data_mut() = Symbol::with_func_symbol("+");
                root.push_back(tr(Symbol::with_func_symbol("-")) / second);
                root.evaluate(level)
            } else if root.front().unwrap().data().is_symbol_name("*") {
                let mut child = root.pop_front().unwrap();
                swap_node(root, &mut child.root_mut());
                let first_arg = root.pop_front().unwrap();
                root.push_front(tr(Symbol::with_func_symbol("-")) / first_arg);
                root.evaluate(level)
            } else {
                remove_zeroes(root)
            }
        }
    }
}

fn remove_zeroes(root: &mut SymbolNode) -> bool {
    match root.degree() {
        1 => {
            if let Symbol::Number(d) = root.back().unwrap().data() {
                if d.is_zero() {
                    swap_node(root, &mut tr(Symbol::Number(0.into())).root_mut());
                    return true;
                }
            }
            false
        }
        2 => match (&root.front().unwrap().data(), &root.back().unwrap().data()) {
            (Symbol::Number(d1), Symbol::Number(d2)) if d1.is_zero() && d2.is_zero() => {
                swap_node(root, &mut tr(Symbol::Number(0.into())).root_mut());
                true
            }
            (Symbol::Number(d), _) if d.is_zero() => {
                let _ = root.pop_front().unwrap();
                true
            }
            (_, Symbol::Number(d)) if d.is_zero() => {
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
    use crate::symbol::func::base::calculator_check;

    #[test]
    fn calculator_test() {
        let (source, level_one, level_two, level_all) = ("-3", "-3", "-3", "-3");
        calculator_check(source, source, minus, NormalizationLevel(0));
        calculator_check(source, level_one, minus, NormalizationLevel(1));
        calculator_check(source, level_two, minus, NormalizationLevel(2));
        calculator_check(source, level_all, minus, NormalizationLevel::max());
    }
}

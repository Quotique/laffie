use std::{cmp::Ordering, rc::Rc};

use bigdecimal::{BigDecimal as Decimal, Zero};
use trees::{tr, Node};

#[cfg(test)]
use crate::term::term_with_params;
use crate::{
    term::{NodeMapping, Symbol, SymbolAttr, TermNode, TermTree},
    NormalizationLevel,
};

pub mod divide;
pub mod equal;
pub mod inequal;
pub mod is;
pub mod less;
pub mod less_or_equal;
pub mod minus;
pub mod more;
pub mod more_or_equal;
pub mod mul;
pub mod op_not;
pub mod op_true;
pub mod plus;
pub mod power;
pub mod replace;
pub mod sqrt;
pub mod symbolic_eq;

pub const MAX_DEC_CONVERSION_EXP: i64 = 6;

fn associative_nesting_remove(root: &mut Node<Symbol>) -> bool {
    let mut result = false;
    if let Some(symbol) = &root.data().func_symbol() {
        if symbol.attrs.read().contains_key(&SymbolAttr::Associative) {
            let root_degree = root.degree();
            for _ in 0..root_degree {
                let mut child = root.pop_front().unwrap();
                if let Some(child_symbol) = &child.data().func_symbol() {
                    if child_symbol == symbol {
                        while let Some(node) = child.pop_front() {
                            root.push_back(node);
                        }
                        result = true;
                        continue;
                    }
                }
                root.push_back(child);
            }
        }
    }
    result
}

fn default_ordering(left: &TermNode, right: &TermNode) -> Ordering {
    // Symbol < Param < Varible < Number < Placeholder
    match (left.data(), right.data()) {
        (Symbol::FuncSymbol(id_l), Symbol::FuncSymbol(id_r)) => id_l.cmp(id_r),
        (Symbol::FuncSymbol(_), _) => Ordering::Less,

        (Symbol::Param(id_l), Symbol::Param(id_r)) => id_l.cmp(id_r),
        (Symbol::Param(_), Symbol::FuncSymbol(_)) => Ordering::Greater,
        (Symbol::Param(_), _) => Ordering::Less,

        (Symbol::Variable(id_l), Symbol::Variable(id_r)) => id_l.cmp(id_r),
        (Symbol::Variable(_), Symbol::Number(_)) => Ordering::Less,
        (Symbol::Variable(_), Symbol::Placeholder(_)) => Ordering::Less,
        (Symbol::Variable(_), _) => Ordering::Greater,

        (Symbol::Number(d1), Symbol::Number(d2)) => d1.cmp(d2),
        (Symbol::Number(_), Symbol::Placeholder(_)) => Ordering::Less,
        (Symbol::Number(_), _) => Ordering::Greater,

        (Symbol::Placeholder(_), Symbol::Placeholder(_)) => Ordering::Equal,
        (Symbol::Placeholder(_), _) => Ordering::Greater,
    }
}

fn commutative_reorder(root: &mut Node<Symbol>) -> bool {
    let mut result = false;
    if let Some(symbol) = &root.data().func_symbol() {
        if symbol.attrs.read().contains_key(&SymbolAttr::Commutative) {
            // TODO: replace with is_sorted_by when it's stable
            // https://doc.rust-lang.org/std/vec/struct.Vec.html#method.is_sorted_by
            let mut sorted = true;
            root.iter().reduce(|prev, x| {
                if symbol
                    .arg_order(prev, x)
                    .unwrap_or_else(|| default_ordering(prev, x)) ==
                    Ordering::Greater
                {
                    sorted = false;
                }
                x
            });
            if !sorted {
                result = true;
                let mut to_sort = vec![];

                while let Some(t) = root.pop_front() {
                    to_sort.push(Rc::new(t));
                }

                to_sort.sort_by(|x, y| {
                    symbol
                        .arg_order(x.root(), y.root())
                        .unwrap_or_else(|| default_ordering(x.root(), y.root()))
                });

                while let Some(t) = to_sort.pop() {
                    root.push_front(Rc::try_unwrap(t).unwrap());
                }
            }
        }
    }
    result
}

pub fn normalize(root: &mut TermNode, level: NormalizationLevel) -> bool {
    let mut result = false;
    for mut i in root.iter_mut() {
        result |= normalize(&mut i, level);
    }

    result |= associative_nesting_remove(root);
    // result |= commutative_reorder(root);
    result |= root.evaluate(level);
    if level > NormalizationLevel(0) {
        result |= commutative_reorder(root); // TODO: reorder once
    }
    result
}

fn from_const(d: Decimal) -> TermTree {
    if d < Decimal::zero() {
        tr(Symbol::with_func_symbol("-")) / tr(Symbol::Number(-d))
    } else {
        tr(Symbol::Number(d))
    }
}

fn to_const(node: &TermNode) -> Option<Decimal> {
    if let Some(d) = node.data().number() {
        Some(d.clone())
    } else if node.data().is_symbol_name("-") {
        node.front().unwrap().data().number().map(|d| -d.clone())
    } else {
        None
    }
}

fn compare_numbers(left: &TermNode, right: &TermNode) -> Option<Ordering> {
    let left_num = to_const(left)?;
    let right_num = to_const(right)?;

    Some(left_num.cmp(&right_num))
}

#[cfg(test)]
pub fn calculator_check(
    src: &'static str,
    res: &'static str,
    f: impl Fn(&mut TermNode, NormalizationLevel) -> bool,
    level: NormalizationLevel,
) {
    let mut s = term_with_params(src);
    let r = term_with_params(res);
    assert_eq!(
        f(&mut s.root_mut(), level),
        src != res,
        "{src} {res} l:{level}"
    );
    assert_eq!(s, r, "{src} {res} l:{level}");
}

#[cfg(test)]
mod operations_tests {
    use bigdecimal::{BigDecimal as Decimal, Num};
    use trees::tr;

    use super::*;

    #[test]
    fn associative_nesting_remove_test() {
        // (1+2)+(1+2) -> 1+2+1+2
        let mut test_tree = tr(Symbol::with_func_symbol("+")) /
            (tr(Symbol::with_func_symbol("+")) /
                tr(Symbol::with_number(1)) /
                tr(Symbol::with_number(2))) /
            (tr(Symbol::with_func_symbol("+")) /
                tr(Symbol::with_number(1)) /
                tr(Symbol::with_number(2)));
        assert!(associative_nesting_remove(&mut test_tree.root_mut()));
        assert_eq!(test_tree.root().degree(), 4);
    }

    #[test]
    fn evaluate_plus_test() {
        // 1+2+5 -> 8
        let mut test_tree1 = tr(Symbol::with_func_symbol("+")) /
            tr(Symbol::with_number(1)) /
            tr(Symbol::with_number(2)) /
            tr(Symbol::with_number(5));
        assert!(test_tree1.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree1, tr(Symbol::Number(Decimal::from(8))));

        // x+1+2+5 -> x+8
        let mut test_tree1 = tr(Symbol::with_func_symbol("+")) /
            tr(Symbol::Variable("x".parse().unwrap())) /
            tr(Symbol::with_number(1)) /
            tr(Symbol::with_number(2)) /
            tr(Symbol::with_number(5));
        assert!(test_tree1.root_mut().evaluate(NormalizationLevel::max()));
        commutative_reorder(&mut test_tree1.root_mut());
        assert_eq!(
            test_tree1,
            tr(Symbol::with_func_symbol("+")) /
                tr(Symbol::Variable("x".parse().unwrap())) /
                tr(Symbol::with_number(8))
        );
    }

    #[test]
    fn evaluate_multiply_test() {
        // 1*2*5 -> 10
        let mut test_tree = tr(Symbol::with_func_symbol("*")) /
            tr(Symbol::with_number(1)) /
            tr(Symbol::with_number(2)) /
            tr(Symbol::with_number(5));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, tr(Symbol::Number(Decimal::from(10))));

        // x*1*2*5 -> 10*x
        let mut test_tree = tr(Symbol::with_func_symbol("*")) /
            tr(Symbol::Variable("x".parse().unwrap())) /
            tr(Symbol::with_number(1)) /
            tr(Symbol::with_number(2)) /
            tr(Symbol::with_number(5));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        commutative_reorder(&mut test_tree.root_mut());
        assert_eq!(
            test_tree,
            tr(Symbol::with_func_symbol("*")) /
                tr(Symbol::with_number(10)) /
                tr(Symbol::Variable("x".parse().unwrap()))
        );

        // x*1 -> x
        let mut test_tree = tr(Symbol::with_func_symbol("*")) /
            tr(Symbol::Variable("x".parse().unwrap())) /
            tr(Symbol::with_number(1));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, tr(Symbol::Variable("x".parse().unwrap())));
    }

    #[test]
    fn evaluate_minus_test() {
        // 10 - 2 -> 8
        let mut test_tree = tr(Symbol::with_func_symbol("-")) /
            tr(Symbol::with_number(10)) /
            tr(Symbol::with_number(2));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, tr(Symbol::with_number(8)));

        // x - 2 -> x + (- 2)
        let mut test_tree = tr(Symbol::with_func_symbol("-")) /
            tr(Symbol::Variable("x".parse().unwrap())) /
            tr(Symbol::with_number(2));
        assert!(!test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(
            test_tree,
            tr(Symbol::with_func_symbol("+")) /
                tr(Symbol::Variable("x".parse().unwrap())) /
                (tr(Symbol::with_func_symbol("-")) / tr(Symbol::with_number(2)))
        );
    }

    #[test]
    fn evaluate_divide_test() {
        // 10 / 2 -> 5
        let mut test_tree = tr(Symbol::with_func_symbol("/")) /
            tr(Symbol::with_number(10)) /
            tr(Symbol::with_number(2));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, tr(Symbol::with_number(5)));

        // x / 2 -> x / 2
        let mut test_tree = tr(Symbol::with_func_symbol("/")) /
            tr(Symbol::Variable("x".parse().unwrap())) /
            tr(Symbol::with_number(2));
        assert!(!test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(
            test_tree,
            tr(Symbol::with_func_symbol("/")) /
                tr(Symbol::Variable("x".parse().unwrap())) /
                tr(Symbol::with_number(2))
        );

        // 2 / 5 -> 0.4
        let mut test_tree = tr(Symbol::with_func_symbol("/")) /
            tr(Symbol::with_number(2)) /
            tr(Symbol::with_number(5));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(
            test_tree,
            tr(Symbol::Number(Decimal::from_str_radix("0.4", 10).unwrap()))
        );

        // 30 / 45 -> 2/3
        let mut test_tree = tr(Symbol::with_func_symbol("/")) /
            tr(Symbol::with_number(30)) /
            tr(Symbol::with_number(45));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(
            test_tree,
            tr(Symbol::with_func_symbol("/")) /
                tr(Symbol::with_number(2)) /
                tr(Symbol::with_number(3))
        );

        // 30 / 4.5 -> 20/3
        let mut test_tree = tr(Symbol::with_func_symbol("/")) /
            tr(Symbol::with_number(30)) /
            tr(Symbol::with_number((45, 1)));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(
            test_tree,
            tr(Symbol::with_func_symbol("/")) /
                tr(Symbol::with_number(20)) /
                tr(Symbol::with_number(3))
        );
    }

    #[test]
    fn evaluate_power_test() {
        // 2 ^ 2 -> 4
        let mut test_tree = tr(Symbol::with_func_symbol("^")) /
            tr(Symbol::with_number(2)) /
            tr(Symbol::with_number(2));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, tr(Symbol::with_number(4)));

        // 2 ^ (-2) -> 0.25
        let mut test_tree = tr(Symbol::with_func_symbol("^")) /
            tr(Symbol::with_number(2)) /
            tr(Symbol::with_number(-2));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, tr(Symbol::with_number((25, 2))));

        // 0.5 ^ (-2) -> 4
        let mut test_tree = tr(Symbol::with_func_symbol("^")) /
            tr(Symbol::with_number((5, 1))) /
            tr(Symbol::with_number(-2));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, tr(Symbol::with_number(4)));

        // 3 ^ (-2) -> 1/9
        let mut test_tree = tr(Symbol::with_func_symbol("^")) /
            tr(Symbol::with_number(3)) /
            tr(Symbol::with_number(-2));
        assert!(test_tree.root_mut().evaluate(NormalizationLevel::max()));
        assert_eq!(
            test_tree,
            tr(Symbol::with_func_symbol("/")) /
                tr(Symbol::with_number(1)) /
                tr(Symbol::with_number(9))
        );
    }

    #[test]
    fn commutative_reorder_test() {
        // 1+2+5+(2*x)+x+(2+3) -> (2+3)+(2*x)+x+1+2+5
        let mut test_tree = tr(Symbol::with_func_symbol("+")) /
            tr(Symbol::with_number(1)) /
            tr(Symbol::with_number(2)) /
            tr(Symbol::with_number(5)) /
            (tr(Symbol::with_func_symbol("*")) /
                tr(Symbol::with_number(2)) /
                tr(Symbol::Variable("x".parse().unwrap()))) /
            tr(Symbol::Variable("x".parse().unwrap())) /
            (tr(Symbol::with_func_symbol("+")) /
                tr(Symbol::with_number(2)) /
                tr(Symbol::with_number(3)));

        assert!(commutative_reorder(test_tree.root_mut().get_mut()));
        assert_eq!(
            test_tree,
            tr(Symbol::with_func_symbol("+")) /
                (tr(Symbol::with_func_symbol("+")) /
                    tr(Symbol::with_number(2)) /
                    tr(Symbol::with_number(3))) /
                (tr(Symbol::with_func_symbol("*")) /
                    tr(Symbol::with_number(2)) /
                    tr(Symbol::Variable("x".parse().unwrap()))) /
                tr(Symbol::Variable("x".parse().unwrap())) /
                tr(Symbol::with_number(1)) /
                tr(Symbol::with_number(2)) /
                tr(Symbol::with_number(5))
        );
    }
}

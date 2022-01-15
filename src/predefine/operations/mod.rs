use std::{cmp::Ordering, rc::Rc};

use trees::Node;

use statement::{
    symbols::SymbolAttr,
    term::{StatementNode, Term},
    tree_utils::NodeMapping,
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
pub mod op_true;
pub mod plus;
pub mod power;
pub mod replace;
pub mod sqrt;

pub const MAX_DEC_CONVERSION_VALUE: i64 = 1_000_000;
pub const MAX_DEC_CONVERSION_EXP: i64 = 6;

fn associative_nesting_remove(root: &mut Node<Term>) -> bool {
    let mut result = false;
    if let Some(symbol) = &root.data().symbol() {
        if symbol.attrs.contains_key(&SymbolAttr::Associative) {
            let root_degree = root.degree();
            for _ in 0..root_degree {
                let mut child = root.pop_front().unwrap();
                if let Some(child_symbol) = &child.data().symbol() {
                    if child_symbol.id == symbol.id {
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

fn default_ordering(left: &StatementNode, right: &StatementNode) -> Ordering {
    // Symbol < Param < Varible < Number < Placeholder
    match (left.data(), right.data()) {
        (Term::Symbol(id_l), Term::Symbol(id_r)) => id_l.cmp(id_r),
        (Term::Symbol(_), _) => Ordering::Less,

        (Term::Param(id_l), Term::Param(id_r)) => id_l.cmp(id_r),
        (Term::Param(_), Term::Symbol(_)) => Ordering::Greater,
        (Term::Param(_), _) => Ordering::Less,

        (Term::Variable(id_l), Term::Variable(id_r)) => id_l.cmp(id_r),
        (Term::Variable(_), Term::Number(_)) => Ordering::Less,
        (Term::Variable(_), Term::Placeholder(_)) => Ordering::Less,
        (Term::Variable(_), _) => Ordering::Greater,

        (Term::Number(d1), Term::Number(d2)) => d1.cmp(d2),
        (Term::Number(_), Term::Placeholder(_)) => Ordering::Less,
        (Term::Number(_), _) => Ordering::Greater,

        (Term::Placeholder(_), Term::Placeholder(_)) => Ordering::Equal,
        (Term::Placeholder(_), _) => Ordering::Greater,
    }
}

fn commutative_reorder(root: &mut Node<Term>) -> bool {
    let mut result = false;
    if let Some(symbol) = &root.data().symbol() {
        if symbol.attrs.contains_key(&SymbolAttr::Commutative) {
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
    result
}

pub fn normalize(root: &mut StatementNode) -> bool {
    let mut result = false;
    for mut i in root.iter_mut() {
        result |= normalize(&mut i);
    }

    result |= associative_nesting_remove(root);
    result |= commutative_reorder(root);
    result |= root.evaluate();
    result |= commutative_reorder(root); // TODO: reorder once

    result
}

#[cfg(test)]
mod operations_tests {
    use bigdecimal::{BigDecimal as Decimal, Num};
    use trees::tr;

    use predefine::setup;
    use statement::symbols::symbol_by_name;

    use super::*;

    #[test]
    fn associative_nesting_remove_test() {
        setup();

        // (1+2)+(1+2) -> 1+2+1+2
        let mut test_tree = tr(Term::Symbol(2)) /
            (tr(Term::Symbol(2)) /
                tr(Term::Number(Decimal::from(1))) /
                tr(Term::Number(Decimal::from(2)))) /
            (tr(Term::Symbol(2)) /
                tr(Term::Number(Decimal::from(1))) /
                tr(Term::Number(Decimal::from(2))));
        assert!(associative_nesting_remove(&mut test_tree.root_mut()));
        assert_eq!(test_tree.root().degree(), 4);
    }

    #[test]
    fn evaluate_plus_test() {
        setup();

        // 1+2+5 -> 8
        let mut test_tree1 = tr(Term::Symbol(2)) /
            tr(Term::Number(Decimal::from(1))) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(5)));
        assert!(test_tree1.root_mut().evaluate());
        assert_eq!(test_tree1, tr(Term::Number(Decimal::from(8))));

        // x+1+2+5 -> x+8
        let mut test_tree1 = tr(Term::Symbol(2)) /
            tr(Term::Variable("x".parse().unwrap())) /
            tr(Term::Number(Decimal::from(1))) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(5)));
        assert!(test_tree1.root_mut().evaluate());
        commutative_reorder(&mut test_tree1.root_mut());
        assert_eq!(
            test_tree1,
            tr(Term::Symbol(2)) /
                tr(Term::Variable("x".parse().unwrap())) /
                tr(Term::Number(Decimal::from(8)))
        );
    }

    #[test]
    fn evaluate_multiply_test() {
        setup();

        // 1*2*5 -> 10
        let mut test_tree = tr(Term::Symbol(7)) /
            tr(Term::Number(Decimal::from(1))) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(5)));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(test_tree, tr(Term::Number(Decimal::from(10))));

        // x*1*2*5 -> 10*x
        let mut test_tree = tr(Term::Symbol(7)) /
            tr(Term::Variable("x".parse().unwrap())) /
            tr(Term::Number(Decimal::from(1))) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(5)));
        assert!(test_tree.root_mut().evaluate());
        commutative_reorder(&mut test_tree.root_mut());
        assert_eq!(
            test_tree,
            tr(Term::Symbol(7)) /
                tr(Term::Number(Decimal::from(10))) /
                tr(Term::Variable("x".parse().unwrap()))
        );

        // x*1 -> x
        let mut test_tree = tr(Term::Symbol(7)) /
            tr(Term::Variable("x".parse().unwrap())) /
            tr(Term::Number(Decimal::from(1)));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(test_tree, tr(Term::Variable("x".parse().unwrap())));
    }

    #[test]
    fn evaluate_minus_test() {
        setup();

        // 10 - 2 -> 8
        let mut test_tree = tr(Term::Symbol(3)) /
            tr(Term::Number(Decimal::from(10))) /
            tr(Term::Number(Decimal::from(2)));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(test_tree, tr(Term::Number(Decimal::from(8))));

        // x - 2 -> x - 2
        let mut test_tree = tr(Term::Symbol(3)) /
            tr(Term::Variable("x".parse().unwrap())) /
            tr(Term::Number(Decimal::from(2)));
        assert!(!test_tree.root_mut().evaluate());
        assert_eq!(
            test_tree,
            tr(Term::Symbol(3)) /
                tr(Term::Variable("x".parse().unwrap())) /
                tr(Term::Number(Decimal::from(2)))
        );
    }

    #[test]
    fn evaluate_divide_test() {
        setup();

        // 10 / 2 -> 5
        let mut test_tree = tr(Term::Symbol(8)) /
            tr(Term::Number(Decimal::from(10))) /
            tr(Term::Number(Decimal::from(2)));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(test_tree, tr(Term::Number(Decimal::from(5))));

        // x / 2 -> x / 2
        let mut test_tree = tr(Term::Symbol(8)) /
            tr(Term::Variable("x".parse().unwrap())) /
            tr(Term::Number(Decimal::from(2)));
        assert!(!test_tree.root_mut().evaluate());
        assert_eq!(
            test_tree,
            tr(Term::Symbol(8)) /
                tr(Term::Variable("x".parse().unwrap())) /
                tr(Term::Number(Decimal::from(2)))
        );

        // 2 / 5 -> 0.4
        let mut test_tree = tr(Term::Symbol(8)) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(5)));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(
            test_tree,
            tr(Term::Number(Decimal::from_str_radix("0.4", 10).unwrap()))
        );

        // 30 / 45 -> 2/3
        let mut test_tree = tr(Term::Symbol(8)) /
            tr(Term::Number(Decimal::from(30))) /
            tr(Term::Number(Decimal::from(45)));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(
            test_tree,
            tr(Term::Symbol(8)) /
                tr(Term::Number(Decimal::from(2))) /
                tr(Term::Number(Decimal::from(3)))
        );

        // 30 / 4.5 -> 2/3
        let mut test_tree = tr(Term::Symbol(8)) /
            tr(Term::Number(Decimal::from(30))) /
            tr(Term::Number(Decimal::from((45.into(), 1))));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(
            test_tree,
            tr(Term::Symbol(8)) /
                tr(Term::Number(Decimal::from(20))) /
                tr(Term::Number(Decimal::from(3)))
        );
    }

    #[test]
    fn evaluate_power_test() {
        setup();

        let power_sym = symbol_by_name("^").unwrap();
        let div_sym = symbol_by_name("/").unwrap();

        // 2 ^ 2 -> 4
        let mut test_tree = tr(Term::Symbol(power_sym.id)) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(2)));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(test_tree, tr(Term::Number(Decimal::from(4))));

        // 2 ^ (-2) -> 0.25
        let mut test_tree = tr(Term::Symbol(power_sym.id)) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(-2)));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(test_tree, tr(Term::Number(Decimal::from((25.into(), 2)))));

        // 0.5 ^ (-2) -> 4
        let mut test_tree = tr(Term::Symbol(power_sym.id)) /
            tr(Term::Number(Decimal::from((5.into(), 1)))) /
            tr(Term::Number(Decimal::from(-2)));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(test_tree, tr(Term::Number(Decimal::from(4))));

        // 3 ^ (-2) -> 1/9
        let mut test_tree = tr(Term::Symbol(power_sym.id)) /
            tr(Term::Number(Decimal::from(3))) /
            tr(Term::Number(Decimal::from(-2)));
        assert!(test_tree.root_mut().evaluate());
        assert_eq!(
            test_tree,
            tr(Term::Symbol(div_sym.id)) /
                tr(Term::Number(Decimal::from(1))) /
                tr(Term::Number(Decimal::from(9)))
        );
    }

    #[test]
    fn commutative_reorder_test() {
        setup();

        // 1+2+5+(2*x)+x+(2+3) -> (2+3)+(2*x)+x+1+2+5
        let mut test_tree = tr(Term::Symbol(2)) /
            tr(Term::Number(Decimal::from(1))) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(5))) /
            (tr(Term::Symbol(7)) /
                tr(Term::Number(Decimal::from(2))) /
                tr(Term::Variable("x".parse().unwrap()))) /
            tr(Term::Variable("x".parse().unwrap())) /
            (tr(Term::Symbol(2)) /
                tr(Term::Number(Decimal::from(2))) /
                tr(Term::Number(Decimal::from(3))));

        assert!(commutative_reorder(test_tree.root_mut().get_mut()));
        assert_eq!(
            test_tree,
            tr(Term::Symbol(2)) /
                (tr(Term::Symbol(2)) /
                    tr(Term::Number(Decimal::from(2))) /
                    tr(Term::Number(Decimal::from(3)))) /
                (tr(Term::Symbol(7)) /
                    tr(Term::Number(Decimal::from(2))) /
                    tr(Term::Variable("x".parse().unwrap()))) /
                tr(Term::Variable("x".parse().unwrap())) /
                tr(Term::Number(Decimal::from(1))) /
                tr(Term::Number(Decimal::from(2))) /
                tr(Term::Number(Decimal::from(5)))
        );
    }
}

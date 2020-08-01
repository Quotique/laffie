use bigdecimal::{BigDecimal as Decimal, One, Zero};
use num::integer::gcd;
use num_bigint::{BigInt, ToBigInt};
use std::{cmp::max, rc::Rc};
use trees::{tr, Node};

use core::{
    symbols::{symbol_by_name, SymbolAttr},
    term::{StatementTree, Term},
};

const MAX_DEC_CONVERSION_VALUE: i64 = 1000_000;
const MAX_DEC_CONVERSION_EXP: i64 = 6;

fn evaluate(root: &mut Node<Term>) -> bool {
    let mut result = false;
    if let Some(symbol) = &root.data.symbol() {
        match symbol.name.as_str() {
            "+" => {
                if root.degree() >= 2 {
                    let mut sum = Decimal::from(0);
                    while let Some(last) = root.pop_back() {
                        if let Term::Number(d) = &last.data {
                            sum = sum + d;
                            result = true;
                        } else {
                            root.push_back(last);
                            break;
                        }
                    }
                    if root.degree() == 0 {
                        root.data = Term::Number(sum);
                    } else if !sum.is_zero() {
                        root.push_back(tr(Term::Number(sum)));
                    }
                }
            }
            "*" => {
                let mut mul = Decimal::from(1);
                while let Some(last) = root.pop_back() {
                    if let Term::Number(d) = &last.data {
                        mul = mul * d;
                        result = true;
                    } else {
                        root.push_back(last);
                        break;
                    }
                }
                if root.degree() == 0 {
                    root.data = Term::Number(mul);
                } else if (root.degree() == 1) & mul.is_one() {
                    let mut last = root.onto_iter().next().unwrap().depart();
                    while let Some(x) = last.pop_front() {
                        root.push_back(x);
                    }
                    // root.append(last.abandon());
                    root.data = last.data.clone();
                } else if !mul.is_one() {
                    root.push_back(tr(Term::Number(mul)));
                }
            }
            "-" => match root.degree() {
                1 => {
                    let last = root.pop_back().unwrap();
                    if let Term::Number(d) = &last.data {
                        root.data = Term::Number(-d);
                        result = true;
                    } else {
                        root.push_back(last);
                    }
                }
                2 => {
                    if let (Term::Number(d1), Term::Number(d2)) =
                        (&root.first().unwrap().data, &root.last().unwrap().data)
                    {
                        root.data = Term::Number(d1 - d2);
                        result = true;
                        root.pop_back();
                        root.pop_back();
                    }
                }
                _ => {
                    panic!("'-' is binary operator!");
                }
            },
            "/" => {
                if let (Term::Number(d1), Term::Number(d2)) =
                    (&root.first().unwrap().data, &root.last().unwrap().data)
                {
                    let (num_m, num_e) = d1.clone().into_bigint_and_exponent();
                    let (den_m, den_e) = d2.clone().into_bigint_and_exponent();

                    let (num_e, den_e) = (num_e - max(num_e, den_e), den_e - max(num_e, den_e));
                    let (num_m, den_m) = (
                        num_m *
                            Decimal::new(1.into(), num_e)
                                .to_bigint()
                                .expect("Unable to get bigint"),
                        den_m *
                            Decimal::new(1.into(), den_e)
                                .to_bigint()
                                .expect("Unable to get bigint"),
                    );

                    let g = gcd(num_m.clone(), den_m.clone());
                    let (num_m, den_m) = (num_m / g.clone(), den_m / g);

                    result = true;
                    root.pop_back();
                    root.pop_back();

                    if den_m.is_one() {
                        root.data = Term::Number(Decimal::from(num_m));
                    } else if (BigInt::from(MAX_DEC_CONVERSION_VALUE) % den_m.clone()).is_zero() {
                        root.data = Term::Number(Decimal::new(
                            num_m * (BigInt::from(MAX_DEC_CONVERSION_VALUE) / den_m),
                            MAX_DEC_CONVERSION_EXP,
                        ));
                    } else {
                        root.push_back(tr(Term::Number(Decimal::from(num_m))));
                        root.push_back(tr(Term::Number(Decimal::from(den_m))));
                    }
                }
            }
            _ => {}
        }
    }
    result
}

fn associative_nesting_remove(root: &mut Node<Term>) -> bool {
    let mut result = false;
    if let Some(symbol) = &root.data.symbol() {
        if symbol.attrs.contains_key(&SymbolAttr::Associative) {
            let root_degree = root.degree();
            for _ in 0..root_degree {
                let mut child = root.pop_front().unwrap();
                if let Some(child_symbol) = &child.data.symbol() {
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
            // for mut child in root.forest_mut().onto_iter() {
            //     if let Some(child_symbol) = &child.data.symbol() {
            //         if child_symbol.id == symbol.id {
            //             println!("Hui22 {} {} {}", root, root.degree(),
            // root.node_count());             println!("Hui221 {}
            // {} {}", child_symbol, child.degree(), child.node_count());
            //             while let Some(node) = child.pop_front() {
            //                 println!("Hui23 {} {} {}", root, root.degree(),
            // root.node_count());                 println!("Hui231
            // {} {} {}", child_symbol, child.degree(), child.node_count());
            //                 child.insert_before(node);
            //                 println!("Hui232 {} {} {}", root, root.degree(),
            // root.node_count());             }
            //             println!("Hui24 {} {} {}", root, root.degree(),
            // root.node_count());             println!("Hui241 {}
            // {} {}", child_symbol, child.degree(), child.node_count());
            //             child.depart();
            //             println!("Hui25 {} {} {}", root, root.degree(),
            // root.node_count());             result = true;
            //             continue;
            //         }
            //     }
            // }
        }
    }
    result
}

fn commutative_reorder(root: &mut Node<Term>) -> bool {
    let mut result = false;
    if let Some(symbol) = &root.data.symbol() {
        if symbol.attrs.contains_key(&SymbolAttr::Commutative) {
            result = true;
            let mut to_sort = vec![];

            while let Some(t) = root.pop_front() {
                to_sort.push(Rc::new(t));
            }
            // Symbol < Param < Varible < Number
            to_sort.sort_by(|x, y| match &x.data {
                Term::Symbol(id_l) => match &y.data {
                    Term::Symbol(id_r) => id_l.cmp(id_r),
                    _ => std::cmp::Ordering::Less,
                },
                Term::Param(id_l) => match &y.data {
                    Term::Symbol(_) => std::cmp::Ordering::Greater,
                    Term::Param(id_r) => id_l.cmp(id_r),
                    _ => std::cmp::Ordering::Less,
                },
                Term::Variable(id_l) => match &y.data {
                    Term::Variable(id_r) => id_l.cmp(id_r),
                    Term::Number(_) => std::cmp::Ordering::Less,
                    _ => std::cmp::Ordering::Greater,
                },
                Term::Number(d1) => match &y.data {
                    Term::Number(d2) => d1.cmp(d2),
                    _ => std::cmp::Ordering::Greater,
                },
            });
            while let Some(t) = to_sort.pop() {
                root.push_front(Rc::try_unwrap(t).unwrap());
            }
        }
    }
    result
}

pub fn normalize(root: &mut Node<Term>) -> bool {
    let mut result = false;
    for i in root.iter_mut() {
        result = result | normalize(i);
    }

    result = result | associative_nesting_remove(root);
    result = result | commutative_reorder(root);
    result = result | evaluate(root);

    result
}

pub fn is_true(statement: &StatementTree) -> bool {
    if let Term::Symbol(id) = &statement.data {
        if *id == symbol_by_name(&"==".into()).unwrap().id {
            if let (Term::Number(d1), Term::Number(d2)) = (
                &statement.first().unwrap().data,
                &statement.last().unwrap().data,
            ) {
                return d1 == d2;
            }
        } else if *id == symbol_by_name(&"!=".into()).unwrap().id {
            if let (Term::Number(d1), Term::Number(d2)) = (
                &statement.first().unwrap().data,
                &statement.last().unwrap().data,
            ) {
                return d1 != d2;
            }
        } else if *id == symbol_by_name(&"<".into()).unwrap().id {
            if let (Term::Number(d1), Term::Number(d2)) = (
                &statement.first().unwrap().data,
                &statement.last().unwrap().data,
            ) {
                return d1 < d2;
            }
        } else if *id == symbol_by_name(&"<=".into()).unwrap().id {
            if let (Term::Number(d1), Term::Number(d2)) = (
                &statement.first().unwrap().data,
                &statement.last().unwrap().data,
            ) {
                return d1 <= d2;
            }
        } else if *id == symbol_by_name(&">".into()).unwrap().id {
            if let (Term::Number(d1), Term::Number(d2)) = (
                &statement.first().unwrap().data,
                &statement.last().unwrap().data,
            ) {
                return d1 > d2;
            }
        } else if *id == symbol_by_name(&">=".into()).unwrap().id {
            if let (Term::Number(d1), Term::Number(d2)) = (
                &statement.first().unwrap().data,
                &statement.last().unwrap().data,
            ) {
                return d1 >= d2;
            }
        } else if *id == symbol_by_name(&"is".into()).unwrap().id {
            if let (Term::Number(_), Term::Symbol(known_id)) = (
                &statement.first().unwrap().data,
                &statement.last().unwrap().data,
            ) {
                return known_id == &symbol_by_name(&"known".into()).unwrap().id;
            }
        } else if *id == symbol_by_name(&"true".into()).unwrap().id {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod operations_tests {
    use super::*;
    use bigdecimal::{BigDecimal as Decimal, Num};
    use core::symbols::symbols_tests::setup;
    use trees::tr;

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
        assert!(evaluate(&mut test_tree1.root_mut()));
        assert_eq!(test_tree1, tr(Term::Number(Decimal::from(8))));

        // x+1+2+5 -> x+8
        let mut test_tree1 = tr(Term::Symbol(2)) /
            tr(Term::Variable(1)) /
            tr(Term::Number(Decimal::from(1))) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(5)));
        assert!(evaluate(&mut test_tree1.root_mut()));
        assert_eq!(
            test_tree1,
            tr(Term::Symbol(2)) / tr(Term::Variable(1)) / tr(Term::Number(Decimal::from(8)))
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
        assert!(evaluate(&mut test_tree.root_mut()));
        assert_eq!(test_tree, tr(Term::Number(Decimal::from(10))));

        // x*1*2*5 -> x*10
        let mut test_tree = tr(Term::Symbol(7)) /
            tr(Term::Variable(1)) /
            tr(Term::Number(Decimal::from(1))) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(5)));
        assert!(evaluate(&mut test_tree.root_mut()));
        assert_eq!(
            test_tree,
            tr(Term::Symbol(7)) / tr(Term::Variable(1)) / tr(Term::Number(Decimal::from(10)))
        );

        // x*1 -> x
        let mut test_tree =
            tr(Term::Symbol(7)) / tr(Term::Variable(1)) / tr(Term::Number(Decimal::from(1)));
        assert!(evaluate(&mut test_tree.root_mut()));
        assert_eq!(test_tree, tr(Term::Variable(1)));
    }

    #[test]
    fn evaluate_minus_test() {
        setup();

        // 10 - 2 -> 8
        let mut test_tree = tr(Term::Symbol(3)) /
            tr(Term::Number(Decimal::from(10))) /
            tr(Term::Number(Decimal::from(2)));
        assert!(evaluate(&mut test_tree.root_mut()));
        assert_eq!(test_tree, tr(Term::Number(Decimal::from(8))));

        // x - 2 -> x - 2
        let mut test_tree =
            tr(Term::Symbol(3)) / tr(Term::Variable(1)) / tr(Term::Number(Decimal::from(2)));
        assert!(!evaluate(&mut test_tree.root_mut()));
        assert_eq!(
            test_tree,
            tr(Term::Symbol(3)) / tr(Term::Variable(1)) / tr(Term::Number(Decimal::from(2)))
        );
    }

    #[test]
    fn evaluate_divide_test() {
        setup();

        // 10 / 2 -> 5
        let mut test_tree = tr(Term::Symbol(8)) /
            tr(Term::Number(Decimal::from(10))) /
            tr(Term::Number(Decimal::from(2)));
        assert!(evaluate(&mut test_tree.root_mut()));
        assert_eq!(test_tree, tr(Term::Number(Decimal::from(5))));

        // x / 2 -> x / 2
        let mut test_tree =
            tr(Term::Symbol(8)) / tr(Term::Variable(1)) / tr(Term::Number(Decimal::from(2)));
        assert!(!evaluate(&mut test_tree.root_mut()));
        assert_eq!(
            test_tree,
            tr(Term::Symbol(8)) / tr(Term::Variable(1)) / tr(Term::Number(Decimal::from(2)))
        );

        // 2 / 5 -> 0.4
        let mut test_tree = tr(Term::Symbol(8)) /
            tr(Term::Number(Decimal::from(2))) /
            tr(Term::Number(Decimal::from(5)));
        assert!(evaluate(&mut test_tree.root_mut()));
        assert_eq!(
            test_tree,
            tr(Term::Number(Decimal::from_str_radix("0.4", 10).unwrap()))
        );

        // 30 / 45 -> 2/3
        let mut test_tree = tr(Term::Symbol(8)) /
            tr(Term::Number(Decimal::from(30))) /
            tr(Term::Number(Decimal::from(45)));
        assert!(evaluate(&mut test_tree.root_mut()));
        assert_eq!(
            test_tree,
            tr(Term::Symbol(8)) /
                tr(Term::Number(Decimal::from(2))) /
                tr(Term::Number(Decimal::from(3)))
        );

        // 30 / 4.5 -> 2/3
        let mut test_tree = tr(Term::Symbol(8)) /
            tr(Term::Number(Decimal::from(30))) /
            tr(Term::Number(Decimal::from(4.5)));
        assert!(evaluate(&mut test_tree.root_mut()));
        assert_eq!(
            test_tree,
            tr(Term::Symbol(8)) /
                tr(Term::Number(Decimal::from(20))) /
                tr(Term::Number(Decimal::from(3)))
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
            (tr(Term::Symbol(7)) / tr(Term::Number(Decimal::from(2))) / tr(Term::Variable(1))) /
            tr(Term::Variable(1)) /
            (tr(Term::Symbol(2)) /
                tr(Term::Number(Decimal::from(2))) /
                tr(Term::Number(Decimal::from(3))));

        assert!(commutative_reorder(test_tree.root_mut()));
        assert_eq!(
            test_tree,
            tr(Term::Symbol(2)) /
                (tr(Term::Symbol(2)) /
                    tr(Term::Number(Decimal::from(2))) /
                    tr(Term::Number(Decimal::from(3)))) /
                (tr(Term::Symbol(7)) /
                    tr(Term::Number(Decimal::from(2))) /
                    tr(Term::Variable(1))) /
                tr(Term::Variable(1)) /
                tr(Term::Number(Decimal::from(1))) /
                tr(Term::Number(Decimal::from(2))) /
                tr(Term::Number(Decimal::from(5)))
        );
    }
}

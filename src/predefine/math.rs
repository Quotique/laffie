use std::collections::HashMap;

use bigdecimal::BigDecimal as Decimal;
use trees::{tr, Node, Tree};

use statement::{term::Term, tree_utils::swap_node};

pub fn plus(root: &mut Node<Term>) -> bool {
    if !root.data().is_symbol_name("+") {
        return false;
    }

    let mut result = false;

    let mut constant_mapping = HashMap::new();

    while let Some(mut child) = root.pop_front() {
        let num_const = extract_mul_const(&mut child.root_mut());
        constant_mapping
            .entry(child)
            .and_modify(|e| {
                *e += num_const.clone();
                result = true;
            })
            .or_insert(num_const);
    }

    for (mul, dec) in constant_mapping.into_iter() {
        let arg = merge_mul_const(mul, dec);
        if !arg.data().is_number_value(&Decimal::from(0)) {
            root.push_back(arg);
        }
    }

    if root.degree() == 0 {
        *root.data_mut() = Term::Number(Decimal::from(0));
    } else if root.degree() == 1 {
        let mut child = root.pop_front().unwrap();
        swap_node(root, &mut child.root_mut());
    }

    // TODO: ordering

    result
}

pub fn multiply(root: &mut Node<Term>) -> bool {
    if !root.data().is_symbol_name("*") {
        return false;
    }

    let mut result = false;

    let mut powers_mapping: HashMap<Tree<Term>, Tree<Term>> = HashMap::new();
    let mut constant = Decimal::from(1);

    while let Some(mut child) = root.pop_front() {
        if let Term::Number(d) = child.data() {
            result = true;
            constant *= d;
        } else {
            let power = extract_power(&mut child.root_mut());
            powers_mapping
                .entry(child)
                .and_modify(|p| {
                    let mut new_pow =
                        tr(Term::with_symbol_name("+").unwrap()) / p.clone() / power.clone();
                    swap_node(&mut p.root_mut(), &mut new_pow.root_mut());
                    result = true;
                })
                .or_insert(power);
        }
    }

    if constant != Decimal::from(1) {
        root.push_back(tr(Term::Number(constant)));
    }

    for (elem, mut pow) in powers_mapping.into_iter() {
        plus(&mut pow.root_mut());
        let arg = merge_power(elem, pow);
        if !arg.data().is_number_value(&Decimal::from(1)) {
            root.push_back(arg);
        }
    }

    if root.degree() == 0 {
        *root.data_mut() = Term::Number(Decimal::from(1));
    } else if root.degree() == 1 {
        let mut child = root.pop_front().unwrap();
        swap_node(root, &mut child.root_mut());
    }

    // TODO: ordering

    result
}

fn merge_power(root: Tree<Term>, pow: Tree<Term>) -> Tree<Term> {
    if pow == tr(Term::Number(Decimal::from(1))) {
        return root;
    }

    if pow == tr(Term::Number(Decimal::from(0))) {
        return tr(Term::Number(Decimal::from(1)));
    }

    tr(Term::with_symbol_name("^").unwrap()) / root / pow
}

fn merge_mul_const(mut root: Tree<Term>, d: Decimal) -> Tree<Term> {
    if d == Decimal::from(1) {
        return root;
    }
    if d == Decimal::from(0) {
        return tr(Term::Number(Decimal::from(0)));
    }
    if root.data().is_number_value(&Decimal::from(1)) {
        return tr(Term::Number(d));
    }

    if root.data().is_symbol_name("*") {
        root.push_front(tr(Term::Number(d)));
        root
    } else {
        tr(Term::with_symbol_name("*").unwrap()) / tr(Term::Number(d)) / root
    }
}

fn extract_power(root: &mut Node<Term>) -> Tree<Term> {
    if root.data().is_symbol_name("^") {
        let power = root.pop_back().unwrap();
        let mut arg = root.pop_front().unwrap();
        assert_eq!(root.degree(), 0);

        swap_node(root, &mut arg.root_mut());
        power
    } else {
        tr(Term::Number(Decimal::from(1)))
    }
}

fn extract_mul_const(root: &mut Node<Term>) -> Decimal {
    if let Term::Number(d) = root.data() {
        let result = d.clone();
        *root.data_mut() = Term::Number(Decimal::from(1));
        return result;
    }
    if !root.data().is_symbol_name("*") {
        return Decimal::from(1);
    }
    let constant = root.iter_mut().fold(Decimal::from(1), |prev, mut x| {
        if let Term::Number(d) = x.data() {
            let res = prev * d;
            x.detach();
            res
        } else {
            prev
        }
    });

    if root.degree() == 0 {
        *root.data_mut() = Term::Number(Decimal::from(1));
    } else if root.degree() == 1 {
        let mut child = root.pop_front().unwrap();
        swap_node(root, &mut child.root_mut());
    }
    constant
}

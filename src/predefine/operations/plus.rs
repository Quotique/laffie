use std::collections::HashMap;

use bigdecimal::BigDecimal as Decimal;
use trees::{tr, Tree};

use crate::statement::{
    symbols::{Symbol, SymbolAttr, SymbolAttrValue},
    term::{StatementNode, Term},
    tree_utils::swap_node,
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("+")
        .with_attr(SymbolAttr::Associative, SymbolAttrValue::None)
        .with_attr(SymbolAttr::Commutative, SymbolAttrValue::None)
        .with_calculator(Box::new(plus))
        .build()
        .unwrap()
}

pub fn plus(root: &mut StatementNode) -> bool {
    if !root.data().is_symbol_name("+") || root.degree() < 2 {
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

fn extract_mul_const(root: &mut StatementNode) -> Decimal {
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

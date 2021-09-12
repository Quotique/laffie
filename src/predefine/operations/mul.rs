use std::collections::HashMap;

use bigdecimal::BigDecimal as Decimal;
use trees::{tr, Tree};

use crate::statement::{
    symbols::{Symbol, SymbolAttr, SymbolAttrValue},
    term::{StatementNode, Term},
    tree_utils::swap_node,
};

use super::plus::plus;

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("*")
        .with_attr(SymbolAttr::Associative, SymbolAttrValue::None)
        .with_attr(SymbolAttr::Commutative, SymbolAttrValue::None)
        .with_calculator(Box::new(multiply))
        .build()
        .unwrap()
}

pub fn multiply(root: &mut StatementNode) -> bool {
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

fn extract_power(root: &mut StatementNode) -> Tree<Term> {
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

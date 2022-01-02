use std::{cmp::Ordering, collections::HashMap};

use bigdecimal::BigDecimal as Decimal;
use trees::{tr, Tree};

use crate::statement::{
    symbols::{symbol_by_id, Symbol, SymbolAttr, SymbolAttrValue},
    term::{StatementNode, Term},
    tree_utils::swap_node,
};

use super::power::power_argument;

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("+")
        .with_attr(SymbolAttr::Associative, SymbolAttrValue::None)
        .with_attr(SymbolAttr::Commutative, SymbolAttrValue::None)
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(300))
        .with_calculator(Box::new(plus))
        .with_ordering(Box::new(ordering))
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

fn cummulative_power(root: &StatementNode) -> Decimal {
    if root.data().is_symbol_name("^") {
        if let Some(v) = root.back().unwrap().data().number() {
            v.clone()
        } else {
            Decimal::from(1)
        }
    } else if root.data().is_symbol_name("*") {
        let mut result = Decimal::from(0);
        for i in root.iter() {
            match i.data() {
                Term::Number(_) | Term::Placeholder => {}
                Term::Symbol(_) if i.data().is_symbol_name("^") => {
                    result = result +
                        if let Some(v) = i.back().unwrap().data().number() {
                            v.clone()
                        } else {
                            Decimal::from(1)
                        };
                }
                Term::Symbol(_) | Term::Variable(_) | Term::Param(_) => {
                    result = result + Decimal::from(1);
                }
            }
        }
        result
    } else {
        Decimal::from(1)
    }
}

fn mean_arg(root: &StatementNode) -> &StatementNode {
    if root.data().is_symbol_name("*") {
        // find first non-number argument and omit power
        // last number argument in non-number not found
        // 4 * x^2 * y -> x
        // 2 * 4 -> 4
        // y * z^4 -> y
        return power_argument(
            root.iter()
                .find(|x| power_argument(x).data().number().is_none())
                .unwrap_or_else(|| root.back().unwrap()),
        );
    }

    power_argument(root)
}

fn ordering(left: &StatementNode, right: &StatementNode) -> Ordering {
    match cummulative_power(left).cmp(&cummulative_power(right)) {
        Ordering::Equal => {}
        Ordering::Less => return Ordering::Greater,
        Ordering::Greater => return Ordering::Less,
    }

    // Symbol < Param < Variable < Number < Placeholder
    match (mean_arg(left).data(), mean_arg(right).data()) {
        (Term::Symbol(left), Term::Symbol(right)) => symbol_by_id(*left)
            .unwrap()
            .name
            .cmp(&symbol_by_id(*right).unwrap().name),
        (Term::Symbol(_), _) => Ordering::Less,

        (Term::Param(left), Term::Param(right)) => left.cmp(right),
        (Term::Param(_), Term::Symbol(_)) => Ordering::Greater,
        (Term::Param(_), _) => Ordering::Less,

        (Term::Variable(left), Term::Variable(right)) => left.cmp(right),
        (Term::Variable(_), Term::Number(_)) => Ordering::Less,
        (Term::Variable(_), Term::Placeholder) => Ordering::Less,
        (Term::Variable(_), _) => Ordering::Greater,

        (Term::Number(left), Term::Number(right)) => left.cmp(right),
        (Term::Number(_), Term::Placeholder) => Ordering::Less,
        (Term::Number(_), _) => Ordering::Greater,

        (Term::Placeholder, Term::Placeholder) => Ordering::Equal,
        (Term::Placeholder, _) => Ordering::Greater,
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::statement_with_vars;

    use super::*;
    #[test]
    fn mean_arg_test() {
        let state = statement_with_vars("4*x^4");
        assert_eq!(mean_arg(state.root()).to_string(), "x".to_owned());

        let state = statement_with_vars("4");
        assert_eq!(mean_arg(state.root()).to_string(), "4".to_owned());

        let state = statement_with_vars("4*2");
        assert_eq!(mean_arg(state.root()).to_string(), "2".to_owned());
    }
}

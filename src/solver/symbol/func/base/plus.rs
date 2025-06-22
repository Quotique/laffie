use std::cmp::Ordering;

use bigdecimal::{BigDecimal as Decimal, Zero};

use crate::{
    symbol::{
        swap_node, FuncSymbol, Symbol, SymbolAttr, SymbolAttrValue, SymbolNode, SymbolNodeMut,
    },
    term::Term,
    NormalizationLevel,
};

use super::power::power_argument;

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("+")
        .with_attr(SymbolAttr::Associative, SymbolAttrValue::None)
        .with_attr(SymbolAttr::Commutative, SymbolAttrValue::None)
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(300))
        .with_calculator(Box::new(plus))
        .with_ordering(Box::new(ordering))
        .build()
}

pub fn plus(root: &mut SymbolNodeMut, level: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("+") || root.degree() < 2 {
        return false;
    }

    match level {
        NormalizationLevel(0) => return false,
        NormalizationLevel(1) => {
            let result = root.iter_mut().fold(false, |acc, mut x| {
                if x.data().is_number_value(&0.into()) {
                    x.detach();
                    true
                } else {
                    acc
                }
            });
            return remove_unused_plus(root) || result;
        }
        NormalizationLevel(2) => {
            let degree = root.degree();
            let (constant, result) = root.iter_mut().enumerate().fold(
                (Decimal::from(0), false),
                |mut acc, (num, mut x)| {
                    if let Some(d) = x.data().number().cloned() {
                        x.detach();
                        acc.1 |= d.is_zero();
                        acc.1 |= degree != num - 1;
                        acc.0 += d;
                    }
                    acc
                },
            );
            attach_constant(root, constant);
            return remove_unused_plus(root) || result;
        }
        _ => {}
    };

    let mut result = false;

    let mut constant_mapping = indexmap::IndexMap::new();

    let mut children: Vec<_> = vec![];
    while let Some(child) = root.pop_front() {
        children.push(child);
    }

    for mut child in children {
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
        } else {
            result = true;
        }
    }

    if root.degree() == 0 {
        *root.data_mut() = Symbol::Number(Decimal::from(0));
        result = true;
    } else if root.degree() == 1 {
        let mut child = root.pop_front().unwrap();
        swap_node(root, &mut child.root_mut());
        result = true;
    }

    result |= super::commutative_reorder(root);

    result
}

fn remove_unused_plus(root: &mut SymbolNodeMut) -> bool {
    if root.degree() == 0 {
        *root.data_mut() = Symbol::Number(Decimal::from(0));
        true
    } else if root.degree() == 1 {
        let mut child = root.pop_front().unwrap();
        swap_node(root, &mut child.root_mut());
        true
    } else {
        false
    }
}

fn attach_constant(root: &mut SymbolNodeMut, constant: Decimal) {
    if !constant.is_zero() {
        root.push_back(Term::number(constant));
    }
}

fn merge_mul_const(mut root: Term, d: Decimal) -> Term {
    if d == Decimal::from(1) {
        return root;
    } else if d == Decimal::from(0) {
        return Term::zero();
    }
    let constant = Term::number(d);

    if root.data().is_number_value(&Decimal::from(1)) {
        return constant;
    }

    if root.data().is_symbol_name("*") {
        root.root_mut().push_front(constant);
        root
    } else {
        Term::func("*").with_child(constant).with_child(root)
    }
}

fn extract_mul_const(root: &mut SymbolNodeMut) -> Decimal {
    if let Some(d) = root.data().number() {
        let result = d.clone();
        swap_node(root, &mut Term::one().root_mut());
        return result;
    }

    let mut constant = Decimal::from(1);
    if !root.data().is_symbol_name("*") {
        return constant;
    }

    let mut children = vec![];
    while let Some(child) = root.pop_front() {
        if let Some(d) = child.data().number() {
            constant *= d;
        } else {
            children.push(child);
        }
    }
    while let Some(child) = children.pop() {
        root.push_front(child);
    }
    // Possible bug in Tree detach: degree stay 2 after detach
    // constant = root.iter_mut().fold(constant, |prev, mut x| {
    //     if let Some(d) = x.data().number() {
    //         let res = prev * d;
    //         x.detach();
    //         res
    //     } else {
    //         prev
    //     }
    // });

    if root.degree() == 0 {
        *root.data_mut() = Symbol::Number(Decimal::from(1));
    } else if root.degree() == 1 {
        let mut child = root.pop_front().unwrap();
        swap_node(root, &mut child.root_mut());
    }
    constant
}

fn cummulative_power(root: SymbolNode) -> Decimal {
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
                Symbol::Number(_) | Symbol::Placeholder(_) => {}
                Symbol::FuncSymbol(_) if i.data().is_symbol_name("^") => {
                    result += if let Some(v) = i.back().unwrap().data().number() {
                        v.clone()
                    } else {
                        Decimal::from(1)
                    };
                }
                Symbol::FuncSymbol(_) | Symbol::Variable(_) | Symbol::Param(_) => {
                    result += Decimal::from(1);
                }
            }
        }
        result
    } else {
        Decimal::from(1)
    }
}

fn mean_arg(root: SymbolNode) -> SymbolNode {
    let pa = if root.data().is_symbol_name("*") {
        // find first non-number argument and omit power
        // last number argument in non-number not found
        // 4 * x^2 * y -> x
        // 2 * 4 -> 4
        // y * z^4 -> y
        power_argument(
            root.iter()
                .find(|x| {
                    let pa = power_argument(*x);
                    if pa.data().number().is_some() {
                        return false;
                    }
                    true
                })
                .unwrap_or_else(|| root.back().unwrap()),
        )
    } else {
        power_argument(root)
    };

    pa
}

fn ordering(left: SymbolNode, right: SymbolNode) -> Ordering {
    match cummulative_power(left).cmp(&cummulative_power(right)) {
        Ordering::Equal => {}
        Ordering::Less => return Ordering::Greater,
        Ordering::Greater => return Ordering::Less,
    }

    // Symbol < Param < Variable < Number < Placeholder
    match (mean_arg(left).data(), mean_arg(right).data()) {
        (Symbol::FuncSymbol(left), Symbol::FuncSymbol(right)) => left.cmp(right),
        (Symbol::FuncSymbol(_), _) => Ordering::Less,

        (Symbol::Param(left), Symbol::Param(right)) => left.cmp(right),
        (Symbol::Param(_), Symbol::FuncSymbol(_)) => Ordering::Greater,
        (Symbol::Param(_), _) => Ordering::Less,

        (Symbol::Variable(left), Symbol::Variable(right)) => left.cmp(right),
        (Symbol::Variable(_), Symbol::Number(_)) => Ordering::Less,
        (Symbol::Variable(_), Symbol::Placeholder(_)) => Ordering::Less,
        (Symbol::Variable(_), _) => Ordering::Greater,

        (Symbol::Number(left), Symbol::Number(right)) => left.cmp(right),
        (Symbol::Number(_), Symbol::Placeholder(_)) => Ordering::Less,
        (Symbol::Number(_), _) => Ordering::Greater,

        (Symbol::Placeholder(_), Symbol::Placeholder(_)) => Ordering::Equal,
        (Symbol::Placeholder(_), _) => Ordering::Greater,
    }
}

#[cfg(test)]
mod tests {
    use crate::{symbol::func::base::calculator_check, term::term_with_vars};

    use super::*;

    #[test]
    fn mean_arg_test() {
        let state = term_with_vars("4*x^4");
        assert_eq!(mean_arg(state.root()).to_string(), "x".to_owned());

        let state = term_with_vars("4");
        assert_eq!(mean_arg(state.root()).to_string(), "4".to_owned());

        let state = term_with_vars("(-1)*y");
        assert_eq!(mean_arg(state.root()).to_string(), "y".to_owned());

        let state = term_with_vars("-y");
        assert_eq!(mean_arg(state.root()).to_string(), "y".to_owned());
    }

    #[test]
    fn plus_test() {
        for (source, level_one, level_two, level_all) in [
            ("x+y", "x+y", "x+y", "x+y"),
            ("0+x", "x", "x", "x"),
            ("1+x", "1+x", "x+1", "x+1"),
            ("0+0", "0", "0", "0"),
            ("1+x+3", "1+x+3", "x+4", "x+4"),
            ("-1+x+3", "-1+x+3", "x+2", "x+2"),
            ("x-y", "x-y", "x-y", "x-y"),
            ("0-x", "-x", "-x", "-x"),
            ("x-0", "x", "x", "x"),
            ("0-0", "0", "0", "0"),
            ("2*x*y+(-1)*x*y", "2*x*y+(-1)*x*y", "2*x*y+(-1)*x*y", "x*y"),
            ("1+2+3", "1+2+3", "6", "6"),
            ("1+2-3", "1+2-3", "0", "0"),
        ] {
            calculator_check(source, source, plus, NormalizationLevel(0));
            calculator_check(source, level_one, plus, NormalizationLevel(1));
            calculator_check(source, level_two, plus, NormalizationLevel(2));
            calculator_check(source, level_all, plus, NormalizationLevel::max());
        }
    }
}

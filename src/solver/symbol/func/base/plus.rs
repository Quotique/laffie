use std::cmp::Ordering;

use bigdecimal::{BigDecimal as Decimal, Zero};
use trees::{tr, Tree};

use crate::{
    symbol::{FuncSymbol, Symbol, SymbolAttr, SymbolAttrValue, SymbolNode},
    term::swap_node,
    NormalizationLevel,
};

use super::{power::power_argument, to_const};

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

pub fn plus(root: &mut SymbolNode, level: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("+") || root.degree() < 2 {
        return false;
    }

    match level {
        NormalizationLevel(0) => return false,
        NormalizationLevel(1) => {
            let result = root.iter_mut().fold(false, |acc, mut x| {
                if x.data().is_number_value(&0.into()) ||
                    (x.data().is_symbol_name("-") &&
                        x.front().unwrap().data().is_number_value(&0.into()))
                {
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
                    } else if x.data().is_symbol_name("-") {
                        if let Some(d) = x.front().unwrap().data().number().cloned() {
                            x.detach();
                            acc.1 |= d.is_zero();
                            acc.1 |= degree != num - 1;
                            acc.0 -= d;
                        }
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

    let mut children = vec![];
    while let Some(mut child) = root.pop_front() {
        if child.data().is_symbol_name("-") && child.degree() == 2 {
            // +(a -(b c)) -> +(a b -c)
            children.push(child.pop_front().unwrap());
        }
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

fn remove_unused_plus(root: &mut SymbolNode) -> bool {
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

fn attach_constant(root: &mut SymbolNode, constant: Decimal) {
    match constant.cmp(&Decimal::zero()) {
        // Ordering::Less => {
        //    root.push_back(tr(Symbol::with_func_symbol("-")) / tr(Symbol::Number(-constant)))
        //}
        Ordering::Less | Ordering::Greater => root.push_back(tr(Symbol::Number(constant))),
        Ordering::Equal => {}
    }
}

fn merge_mul_const(mut root: Tree<Symbol>, d: Decimal) -> Tree<Symbol> {
    if d == Decimal::from(1) {
        return root;
    } else if d == Decimal::from(0) {
        return tr(Symbol::Number(Decimal::from(0)));
        //} else if d == Decimal::from(-1) {
        //    return tr(Symbol::with_func_symbol("-")) / root;
    }
    // let constant = if d < Decimal::zero() {
    //    tr(Symbol::with_func_symbol("-")) / tr(Symbol::Number(-d))
    //} else {
    //    tr(Symbol::Number(d))
    //};
    let constant = tr(Symbol::Number(d));

    if root.data().is_number_value(&Decimal::from(1)) {
        return constant;
    }

    if root.data().is_symbol_name("*") {
        root.push_front(constant);
        root
    } else {
        tr(Symbol::with_func_symbol("*")) / constant / root
    }
}

fn extract_mul_const(root: &mut SymbolNode) -> Decimal {
    if let Some(d) = to_const(root) {
        let result = d.clone();
        swap_node(root, &mut tr(Symbol::Number(1.into())).root_mut());
        return result;
    }

    let base_constant = if root.data().is_symbol_name("-") && root.degree() == 1 {
        let mut child = root.pop_front().unwrap();
        swap_node(root, &mut child.root_mut());
        Decimal::from(-1)
    } else {
        Decimal::from(1)
    };

    if !root.data().is_symbol_name("*") {
        return base_constant;
    }

    let constant = root.iter_mut().fold(base_constant, |prev, mut x| {
        if let Some(d) = to_const(&x) {
            let res = prev * d;
            x.detach();
            res
        } else {
            prev
        }
    });

    if root.degree() == 0 {
        *root.data_mut() = Symbol::Number(Decimal::from(1));
    } else if root.degree() == 1 {
        let mut child = root.pop_front().unwrap();
        swap_node(root, &mut child.root_mut());
    }
    constant
}

fn cummulative_power(root: &SymbolNode) -> Decimal {
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
                Symbol::FuncSymbol(_)
                    if i.data().is_symbol_name("-") &&
                        i.front().unwrap().data().number().is_some() => {}
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

fn mean_arg(root: &SymbolNode) -> &SymbolNode {
    let pa = if root.data().is_symbol_name("*") {
        // find first non-number argument and omit power
        // last number argument in non-number not found
        // 4 * x^2 * y -> x
        // 2 * 4 -> 4
        // y * z^4 -> y
        power_argument(
            root.iter()
                .find(|x| {
                    let pa = power_argument(x);
                    if pa.data().number().is_some() {
                        return false;
                    }
                    if pa.data().is_symbol_name("-") &&
                        pa.front().unwrap().data().number().is_some()
                    {
                        return false;
                    }
                    true
                })
                .unwrap_or_else(|| root.back().unwrap()),
        )
    } else {
        power_argument(root)
    };

    if pa.data().is_symbol_name("-") {
        pa.front().unwrap()
    } else {
        pa
    }
}

fn ordering(left: &SymbolNode, right: &SymbolNode) -> Ordering {
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

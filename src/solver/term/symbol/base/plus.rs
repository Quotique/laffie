use std::{cmp::Ordering, collections::HashMap};

use num::{One, Zero};

use super::{SymbolProgram, power::power_argument};
use crate::{
    NormLevel, Rational,
    term::{Atom, SymbolAttr, SymbolAttrValue, Term, TermBuf, TermMut, TermRef},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "+".into(),
        attrs: HashMap::from([
            (SymbolAttr::Associative, SymbolAttrValue::None),
            (SymbolAttr::Commutative, SymbolAttrValue::None),
            (SymbolAttr::Infix, SymbolAttrValue::UInt(300)),
        ]),
        calculator: Box::new(plus),
        arg_cmp: Box::new(ordering),
        ..Default::default()
    }
}

pub fn plus(root: &mut TermMut, level: NormLevel) -> bool {
    if !root.data().is_symbol_name("+") || root.degree() < 2 {
        return false;
    }

    match level {
        NormLevel::Off => return false,
        NormLevel::Units => {
            let result = root.iter_mut().fold(false, |acc, mut x| {
                if x.data().is_number_value(&Rational::zero()) {
                    x.detach();
                    true
                } else {
                    acc
                }
            });
            return remove_unused_plus(root) || result;
        }
        NormLevel::ConstFold => {
            let degree = root.degree();
            let (constant, result) = root.iter_mut().enumerate().fold(
                (Rational::zero(), false),
                |mut acc, (num, mut x)| {
                    if let Some(d) = x.data().number().cloned() {
                        x.detach();
                        acc.1 |= d.is_zero();
                        acc.1 |= degree != num + 1;
                        acc.0 += d;
                    }
                    acc
                },
            );
            attach_constant(root, constant);
            return remove_unused_plus(root) || result;
        }
        NormLevel::Full => {}
    };

    let mut result = false;

    let mut constant_mapping = indexmap::IndexMap::new();

    let mut children: Vec<_> = vec![];
    while let Some(child) = root.pop_first_arg() {
        children.push(child);
    }

    for mut child in children {
        let num_const = extract_mul_const(&mut child.term_mut());
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
        if !arg.data().is_number_value(&Rational::zero()) {
            root.push_last_arg(arg);
        } else {
            result = true;
        }
    }

    if root.degree() == 0 {
        *root.data_mut() = Atom::Number(Rational::zero());
        result = true;
    } else if root.degree() == 1 {
        let mut child = root.pop_first_arg().unwrap();
        root.swap(&mut child.term_mut());
        result = true;
    }

    // Ordering is the normalization pass's job.
    result
}

fn remove_unused_plus(root: &mut TermMut) -> bool {
    if root.degree() == 0 {
        *root.data_mut() = Atom::Number(Rational::zero());
        true
    } else if root.degree() == 1 {
        let mut child = root.pop_first_arg().unwrap();
        root.swap(&mut child.term_mut());
        true
    } else {
        false
    }
}

fn attach_constant(root: &mut TermMut, constant: Rational) {
    if !constant.is_zero() {
        root.push_last_arg(TermBuf::ratio(constant));
    }
}

fn merge_mul_const(mut root: TermBuf, d: Rational) -> TermBuf {
    if d.is_one() {
        return root;
    } else if d.is_zero() {
        return TermBuf::zero();
    }
    let constant = TermBuf::ratio(d);

    if root.data().is_number_value(&Rational::one()) {
        return constant;
    }

    if root.data().is_symbol_name("*") {
        root.term_mut().push_first_arg(constant);
        root
    } else {
        TermBuf::symbol("*").arg(constant).arg(root)
    }
}

fn extract_mul_const(root: &mut TermMut) -> Rational {
    if let Some(d) = root.data().number() {
        let result = d.clone();
        root.swap(&mut TermBuf::one().term_mut());
        return result;
    }

    let mut constant = Rational::one();
    if !root.data().is_symbol_name("*") {
        return constant;
    }

    let mut children = vec![];
    while let Some(child) = root.pop_first_arg() {
        if let Some(d) = child.data().number() {
            constant *= d;
        } else {
            children.push(child);
        }
    }
    while let Some(child) = children.pop() {
        root.push_first_arg(child);
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
        *root.data_mut() = Atom::Number(Rational::one());
    } else if root.degree() == 1 {
        let mut child = root.pop_first_arg().unwrap();
        root.swap(&mut child.term_mut());
    }
    constant
}

fn cummulative_power(root: TermRef) -> Rational {
    if root.data().is_symbol_name("^") {
        if let Some(v) = root.last_arg().unwrap().data().number() {
            v.clone()
        } else {
            Rational::one()
        }
    } else if root.data().is_symbol_name("*") {
        let mut result = Rational::zero();
        for i in root.args_iter() {
            match i.data() {
                Atom::Number(_) | Atom::ArgList(_) => {}
                Atom::Symbol(_) if i.data().is_symbol_name("^") => {
                    result += if let Some(v) = i.last_arg().unwrap().data().number() {
                        v.clone()
                    } else {
                        Rational::one()
                    };
                }
                Atom::Symbol(_) | Atom::Variable(_) | Atom::Param(_) => {
                    result += Rational::one();
                }
            }
        }
        result
    } else {
        Rational::one()
    }
}

fn mean_arg(root: TermRef) -> TermRef {
    if root.data().is_symbol_name("*") {
        // find first non-number argument and omit power
        // last number argument in non-number not found
        // 4 * x^2 * y -> x
        // 2 * 4 -> 4
        // y * z^4 -> y
        power_argument(
            root.args_iter()
                .find(|x| {
                    let pa = power_argument(*x);
                    if pa.data().number().is_some() {
                        return false;
                    }
                    true
                })
                .unwrap_or_else(|| root.last_arg().unwrap()),
        )
    } else {
        power_argument(root)
    }
}

fn ordering(left: TermRef, right: TermRef) -> Ordering {
    match cummulative_power(left).cmp(&cummulative_power(right)) {
        Ordering::Equal => {}
        Ordering::Less => return Ordering::Greater,
        Ordering::Greater => return Ordering::Less,
    }

    // Symbol < Param < Variable < Number < Placeholder
    match (mean_arg(left).data(), mean_arg(right).data()) {
        (Atom::Symbol(left), Atom::Symbol(right)) => left.cmp(right),
        (Atom::Symbol(_), _) => Ordering::Less,

        (Atom::Param(left), Atom::Param(right)) => left.cmp(right),
        (Atom::Param(_), Atom::Symbol(_)) => Ordering::Greater,
        (Atom::Param(_), _) => Ordering::Less,

        (Atom::Variable(left), Atom::Variable(right)) => left.cmp(right),
        (Atom::Variable(_), Atom::Number(_)) => Ordering::Less,
        (Atom::Variable(_), Atom::ArgList(_)) => Ordering::Less,
        (Atom::Variable(_), _) => Ordering::Greater,

        (Atom::Number(left), Atom::Number(right)) => left.cmp(right),
        (Atom::Number(_), Atom::ArgList(_)) => Ordering::Less,
        (Atom::Number(_), _) => Ordering::Greater,

        (Atom::ArgList(_), Atom::ArgList(_)) => Ordering::Equal,
        (Atom::ArgList(_), _) => Ordering::Greater,
    }
}

#[cfg(test)]
mod tests {
    use crate::term::{symbol::base::calculator_check, term_with_vars};

    use super::*;

    #[test]
    fn mean_arg_test() {
        let state = term_with_vars("4*x^4");
        assert_eq!(mean_arg(state.term()).to_string(), "x".to_owned());

        let state = term_with_vars("4");
        assert_eq!(mean_arg(state.term()).to_string(), "4".to_owned());

        let state = term_with_vars("(-1)*y");
        assert_eq!(mean_arg(state.term()).to_string(), "y".to_owned());

        let state = term_with_vars("-y");
        assert_eq!(mean_arg(state.term()).to_string(), "y".to_owned());
    }

    #[test]
    fn plus_test() {
        // `Full` keeps the term multiset but not a canonical order: `plus` no
        // longer sorts (the normalization pass does).
        for (source, level_one, level_two, level_all) in [
            ("x+y", "x+y", "x+y", "x+y"),
            ("0+x", "x", "x", "x"),
            ("1+x", "1+x", "x+1", "1+x"),
            ("0+0", "0", "0", "0"),
            ("1+x+3", "1+x+3", "x+4", "4+x"),
            ("-1+x+3", "-1+x+3", "x+2", "2+x"),
            ("x-y", "x-y", "x-y", "x-y"),
            ("0-x", "-x", "-x", "-x"),
            ("x-0", "x", "x", "x"),
            ("0-0", "0", "0", "0"),
            ("2*x*y+(-1)*x*y", "2*x*y+(-1)*x*y", "2*x*y+(-1)*x*y", "x*y"),
            ("1+2+3", "1+2+3", "6", "6"),
            ("1+2-3", "1+2-3", "0", "0"),
        ] {
            calculator_check(source, source, plus, NormLevel::Off);
            calculator_check(source, level_one, plus, NormLevel::Units);
            calculator_check(source, level_two, plus, NormLevel::ConstFold);
            calculator_check(source, level_all, plus, NormLevel::Full);
        }
    }
}

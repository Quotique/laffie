use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    iter::{FromIterator, Iterator},
};

use bigdecimal::BigDecimal;
use eyre::{bail, ensure, Result};

use num::Zero;
use utils::VecDisplay;

use crate::term::{swap_node, Param, ParamsMap, Placeholder, Symbol, SymbolAttr, SymbolNodeMut};

use super::{index::NodePosition, SymbolNode, Term};

#[derive(Debug, Clone, Default)]
pub struct ParamsMapping {
    params:       BTreeMap<Param, Term>,
    placeholders: BTreeMap<Placeholder, Vec<Term>>,
}

impl From<ParamsMap> for ParamsMapping {
    fn from(params: ParamsMap) -> Self {
        Self {
            params:       BTreeMap::from_iter(params),
            placeholders: Default::default(),
        }
    }
}

impl ParamsMapping {
    #[inline]
    pub fn params(&self) -> impl Iterator<Item = (&Param, &Term)> {
        self.params.iter()
    }

    #[inline]
    pub fn try_map(target: SymbolNode, pattern: SymbolNode) -> Result<Vec<ParamsMapping>> {
        Self::try_map_with_params(target, pattern, Default::default())
    }

    pub fn subtree_map(
        target: SymbolNode,
        pattern: SymbolNode,
    ) -> Vec<(Vec<ParamsMapping>, NodePosition)> {
        Self::subtree_map_extend(target, pattern, Default::default())
    }

    pub fn subtree_map_extend(
        target: SymbolNode,
        pattern: SymbolNode,
        params: ParamsMapping,
    ) -> Vec<(Vec<ParamsMapping>, NodePosition)> {
        let mut result = vec![];
        let mut queue = VecDeque::new();
        queue.push_back((target, NodePosition::root()));

        while let Some((node, pos)) = queue.pop_front() {
            if let Ok(mapping) = Self::try_map_with_params(node, pattern, params.clone())
                .map_err(|_| trace!(target: "pattern_match", "No match for {pattern} to {node}"))
            {
                result.push((mapping, pos.clone()));
            }

            for (num, i) in node.iter().enumerate() {
                queue.push_back((i, pos.clone().child(num)));
            }
        }
        result
    }

    pub fn apply<'a, 'b>(&self, node: &'b mut SymbolNodeMut<'a>) -> &'b mut SymbolNodeMut<'a> {
        match node.data().clone() {
            Symbol::Param(p) => {
                if let Some(p) = self.params.get(&p) {
                    swap_node(node, &mut p.clone().root_mut());
                }
            }
            Symbol::Placeholder(p) => {
                if let Some(p) = self.placeholders.get(&p) {
                    let mut p = p.clone();
                    swap_node(node, &mut p[0].root_mut());
                    for i in p.into_iter().skip(1).rev() {
                        node.insert_after(i);
                    }
                }
            }
            _ => {
                for mut i in node.iter_mut() {
                    self.apply(&mut i);
                }
            }
        }
        node
    }

    pub fn try_map_with_params(
        target: SymbolNode,
        pattern: SymbolNode,
        mut params: ParamsMapping,
    ) -> Result<Vec<ParamsMapping>> {
        trace!(target: "pattern_match", "Pattern: {pattern}, traget: {target}, mapping: {params:?}");
        let mut result = vec![];

        match (&pattern.data(), &target.data()) {
            (Symbol::FuncSymbol(sym), Symbol::FuncSymbol(t_sym)) => {
                ensure!(sym == t_sym, "Expect symbol {sym}, found: {t_sym}");

                if sym.attrs.read().contains_key(&SymbolAttr::Associative) &&
                    sym.attrs.read().contains_key(&SymbolAttr::Commutative)
                {
                    // TODO: priority mapping
                    // not even trying to map subsets twice
                    for (num, parts) in target.subsets(pattern.degree()).enumerate() {
                        ensure!(num < 1025, "Subsets of operation is too large");
                        let mut loc_result = vec![params.clone()];
                        params_map_arguments(parts.root(), pattern, &mut loc_result)
                            .expect("must match");
                        result.append(&mut loc_result);
                    }
                } else {
                    result.push(params);
                    params_map_arguments(target, pattern, &mut result)?;
                }
                ensure!(!result.is_empty(), "No mapping found");
                return Ok(result);
            }
            // try map (-1)*param on (-number)
            (Symbol::FuncSymbol(mul), Symbol::Number(neg)) if neg < &BigDecimal::zero() => {
                if !pattern.data().is_symbol_name("*") {
                    bail!(
                        "Expect symbol id: {mul}, found target: {:?}",
                        &target.data()
                    );
                }
                let mul_num = Term::func("*")
                    .with_child(Term::number(-1))
                    .with_child(Term::number(neg.abs()));
                return Self::try_map_with_params(mul_num.root(), pattern, params);
            }
            (Symbol::FuncSymbol(p_id), _) => {
                bail!(
                    "Expect symbol id: {p_id}, found target: {:?}",
                    &target.data()
                );
            }
            (Symbol::Param(p), _) => {
                if params.params.contains_key(p) {
                    let node = params.params.get(p).unwrap();
                    let _ = ParamsMapping::try_map(target, node.root())?;
                } else {
                    params.params.insert(p.clone(), target.deep_clone());
                }

                result.push(params);
            }
            (Symbol::Number(value), Symbol::Number(other_value)) => {
                ensure!(
                    value == other_value,
                    "Expect Number {value}, found {:?}",
                    target.data()
                );

                result.push(params);
            }
            (Symbol::Number(_), _) => {
                bail!("Expect Number, found: {:?}", target.data());
            }
            (Symbol::Variable(value), Symbol::Variable(other_value)) => {
                ensure!(
                    value == other_value,
                    "Expect Varible {value}, found {:?}",
                    target.data()
                );

                result.push(params);
            }
            (Symbol::Variable(_), _) => {
                bail!("Expect Varible, found: {:?}", target.data());
            }
            (Symbol::Placeholder(_), _) => {
                bail!("Mapping placeholder")
            }
        }
        Ok(result)
    }
}

fn params_map_arguments(
    target: SymbolNode,
    pattern: SymbolNode,
    result: &mut Vec<ParamsMapping>,
) -> Result<()> {
    let placeholder = pattern
        .iter()
        .enumerate()
        .find_map(|(pos, x)| x.data().placeholder().map(|p| (pos, p)));
    let args_delta = target.degree() as i64 - pattern.degree() as i64;
    ensure!(
        placeholder.is_some() && args_delta >= 0 || placeholder.is_none() && args_delta == 0,
        "Argument size missmatch: {} {}",
        pattern.degree(),
        target.degree()
    );

    for (p, t) in pattern.iter().zip(
        target
            .iter()
            .enumerate()
            .filter(|(num, _)| {
                // Skip placeholder all placeholder args but first.
                if let Some((pos, _)) = placeholder {
                    *num <= pos || *num > 1 + pos + args_delta as usize
                } else {
                    true
                }
            })
            .map(|(_, x)| x),
    ) {
        if p.data().placeholder().is_some() {
            continue;
        }

        let mut new_result = vec![];
        for r in result.drain(..) {
            if let Ok(mut p) = ParamsMapping::try_map_with_params(t, p, r) {
                trace!(target: "pattern_match", "New mapping: [{}]", VecDisplay(&p));
                new_result.append(&mut p);
            }
        }
        *result = new_result;
    }

    if let Some((pos, ph)) = placeholder {
        let mapping: Vec<_> = target
            .iter()
            .enumerate()
            .filter(|(num, _)| *num >= pos && *num < 1 + pos + args_delta as usize)
            .map(|(_, x)| x.deep_clone())
            .collect();

        for i in result.iter_mut() {
            i.placeholders.insert(ph, mapping.clone());
        }
    }

    Ok(())
}

impl fmt::Display for ParamsMapping {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{ {} }}",
            self.params
                .iter()
                .map(|(x, y)| format!("{x}: {y}"))
                .chain(
                    self.placeholders
                        .iter()
                        .map(|(x, y)| format!("..{}: {}", x, VecDisplay(y)))
                )
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::term::{term_with_params, term_with_vars, ParamsMapping};
    use utils::VecDisplay;

    #[test]
    fn simple_param_map_test() {
        let term = term_with_vars("x + 1 == 0");
        let pattern = term_with_params("a + b == 0");

        let maps = ParamsMapping::try_map(term.root(), pattern.root())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: 1, b: x }, { a: x, b: 1 }]");
    }

    #[test]
    fn param_mapping_minus_sign_test() {
        let term = term_with_vars("-x - 5 == 0");
        let pattern = term_with_params("-a + b == 0");

        let maps = ParamsMapping::try_map(term.root(), pattern.root())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: 5, b: -x }, { a: x, b: -5 }]");
    }

    #[test]
    fn param_mapping_minus_sign_2_test() {
        let term = term_with_vars("-x - 5 == 0");
        let pattern = term_with_params("-a - b == 0");

        let maps = ParamsMapping::try_map(term.root(), pattern.root())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: 5, b: x }, { a: x, b: 5 }]");
    }

    #[test]
    fn same_param_map_test() {
        let term = term_with_vars("x + 1 == x");
        let pattern = term_with_params("a + 1 == a");

        let maps = ParamsMapping::try_map(term.root(), pattern.root())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: x }]");
    }

    #[test]
    fn subtree_param_map_test() {
        let term = term_with_vars("2*x^2 + 4 == x - 1");
        let pattern = term_with_params("a + 4 == b");

        let maps = ParamsMapping::try_map(term.root(), pattern.root())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: 2*x^2, b: x-1 }]");
    }

    #[test]
    fn apply_param_map_test() {
        let term = term_with_vars("2*x^2 + 4 == x - 1");
        let pattern = term_with_params("a + 4 == b");

        let maps = ParamsMapping::try_map(term.root(), pattern.root())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        assert_eq!(maps.len(), 1);

        let mut test = term_with_params("a + 1");
        maps[0].apply(&mut test.root_mut());
        insta::assert_debug_snapshot!(test, @"2*x^2+1");
    }

    #[test]
    fn placeholder_test() {
        let pattern = term_with_params("set(a, ..) is known");
        let term = term_with_vars("set(3, 5, 7) is known");

        let maps = ParamsMapping::try_map(term.root(), pattern.root())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: 3, ..1: [5, 7] }]");
    }

    #[test]
    fn placeholder_false_test() {
        let pattern = term_with_params("set(a, ..) is known");
        let term = term_with_vars("set(3) is known");

        assert!(ParamsMapping::try_map(term.root(), pattern.root()).is_err());
    }
}

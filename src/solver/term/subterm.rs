use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    fmt,
    iter::Iterator,
};

use bigdecimal::BigDecimal;
use derive_more::{Debug, From};
use eyre::{bail, ensure, Result};
use itertools::Itertools;
use num::Zero;
use trees::Node;

use utils::{SubsetIterator, VecDisplay};

use super::{index::NodePosition, Param, Placeholder, Symbol, Term, TermNode, Truth};

#[derive(Debug, Clone, Default)]
pub struct ParamsMapping {
    pub params:       BTreeMap<Param, Term>,
    pub placeholders: BTreeMap<Placeholder, Vec<Term>>,
}

#[derive(Clone, Copy)]
#[derive(PartialEq, Eq)]
#[derive(Debug, From)]
pub struct Subterm<'a>(&'a Node<TermNode>);

impl<'a> Subterm<'a> {
    #[inline]
    pub fn degree(&self) -> usize {
        self.0.degree()
    }

    #[inline]
    pub fn data(&self) -> &TermNode {
        self.0.data()
    }

    #[inline]
    pub fn parent(&self) -> Option<Self> {
        self.0.parent().map(Self)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Self> {
        self.0.iter().map(Self)
    }

    #[inline]
    pub fn first_arg(&self) -> Option<Self> {
        self.0.front().map(Subterm)
    }

    #[inline]
    pub fn last_arg(&self) -> Option<Self> {
        self.0.back().map(Subterm)
    }

    #[inline]
    pub fn to_term(&self) -> Term {
        self.0.deep_clone().into()
    }

    #[inline]
    pub fn symbols(&self) -> HashSet<Symbol> {
        self.0.bfs().iter.filter_map(|x| x.data.symbol()).collect()
    }

    pub fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = Term> + '_> {
        Box::new(SubsetIterator::new(self.0.degree(), count).map(move |i| {
            let s = self.0.data().symbol().unwrap();
            let mut parts = vec![Term::from(TermNode::Symbol(s.clone())); count];
            for (id, child) in self.iter().enumerate() {
                parts[i.as_vec()[id]]
                    .as_subterm_mut()
                    .push_last_arg(child.to_term());
            }

            for p in parts.iter_mut() {
                if p.as_subterm().degree() == 1 {
                    let mut child = p.as_subterm_mut().pop_first_arg().unwrap();
                    p.as_subterm_mut().swap(&mut child.as_subterm_mut());
                }
            }
            let mut result = Term::from(TermNode::Symbol(s.clone()));
            for p in parts.into_iter() {
                result.as_subterm_mut().push_last_arg(p);
            }
            result
        }))
    }

    #[inline]
    pub fn truth(&self) -> Truth {
        self.0
            .data()
            .symbol()
            .map(|x| x.check_truth(*self))
            .unwrap_or(Truth::Unknown)
    }

    fn parentheses(&self) -> bool {
        let parent_weight = self
            .parent()
            .and_then(|x| x.data().symbol())
            .and_then(|x| x.display_weight())
            .unwrap_or(u64::MAX);
        let parent_associative = self
            .parent()
            .and_then(|x| x.data().symbol())
            .map(|x| x.is_associative())
            .unwrap_or(false);
        let weight = self
            .data()
            .symbol()
            .and_then(|x| x.display_weight())
            .unwrap_or(u64::MIN);
        weight > parent_weight || (weight == parent_weight && !parent_associative)
    }
}

impl<'a> Subterm<'a> {
    #[inline]
    pub fn try_match(&self, pattern: Subterm) -> Result<Vec<ParamsMapping>> {
        self.try_match_extend(pattern, Default::default())
    }

    pub fn try_subterm_match(&self, pattern: Subterm) -> Vec<(Vec<ParamsMapping>, NodePosition)> {
        self.try_subterm_match_extend(pattern, Default::default())
    }

    pub fn try_subterm_match_extend(
        &self,
        pattern: Subterm,
        params: ParamsMapping,
    ) -> Vec<(Vec<ParamsMapping>, NodePosition)> {
        let mut result = vec![];
        let mut queue = VecDeque::new();
        queue.push_back((*self, NodePosition::root()));

        while let Some((node, pos)) = queue.pop_front() {
            if let Ok(mapping) = node
                .try_match_extend(pattern, params.clone())
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

    pub fn try_match_extend(
        &self,
        pattern: Subterm,
        mut params: ParamsMapping,
    ) -> Result<Vec<ParamsMapping>> {
        trace!(target: "pattern_match", "Pattern: {pattern}, traget: {self}, mapping: {params:?}");

        match (&pattern.data(), &self.data()) {
            (TermNode::Symbol(sym), TermNode::Symbol(t_sym)) => {
                ensure!(sym == t_sym, "Expect symbol {sym}, found: {t_sym}");
                let mut result = vec![];

                if sym.is_associative() && sym.is_commutative() {
                    // TODO: priority mapping
                    // not even trying to map subsets twice
                    for (num, parts) in self.subsets(pattern.degree()).enumerate() {
                        ensure!(num < 1025, "Subsets of operation is too large");

                        let mut loc_result = vec![params.clone()];
                        parts
                            .as_subterm()
                            .try_match_args(pattern, &mut loc_result)
                            .expect("must match");
                        result.append(&mut loc_result);
                    }
                } else {
                    result.push(params);
                    self.try_match_args(pattern, &mut result)?;
                }
                ensure!(!result.is_empty(), "No mapping found");
                Ok(result)
            }
            // try map (-1)*param on (-number)
            (TermNode::Symbol(mul), TermNode::Number(neg))
                if mul == "*" && neg < &BigDecimal::zero() =>
            {
                Term::func("*")
                    .with_child(Term::number(-1))
                    .with_child(Term::number(neg.abs()))
                    .as_subterm()
                    .try_match_extend(pattern, params)
            }
            (TermNode::Symbol(p_id), _) => {
                bail!("Expect symbol id: {p_id}, found target: {:?}", &self.data())
            }
            (TermNode::Param(p), _) => {
                if params.params.contains_key(p) {
                    let node = params.params.get(p).unwrap();
                    let _ = self.try_match(node.as_subterm())?;
                } else {
                    params.params.insert(p.clone(), self.to_term());
                }
                Ok(vec![params])
            }
            (TermNode::Number(value), TermNode::Number(other_value)) if value == other_value => {
                Ok(vec![params])
            }
            (TermNode::Number(value), TermNode::Number(other_value)) => {
                bail!("Expect Number {value}, found {other_value}",)
            }
            (TermNode::Number(_), _) => bail!("Expect Number, found: {:?}", self.data()),
            (TermNode::Variable(value), TermNode::Variable(other_value))
                if value == other_value =>
            {
                Ok(vec![params])
            }
            (TermNode::Variable(value), TermNode::Variable(other_value)) => {
                bail!("Expect Varible {value}, found {other_value}")
            }
            (TermNode::Variable(_), _) => bail!("Expect Varible, found: {:?}", self.data()),
            (TermNode::Placeholder(_), _) => bail!("Mapping placeholder"),
        }
    }

    fn try_match_args(&self, pattern: Subterm, result: &mut Vec<ParamsMapping>) -> Result<()> {
        let placeholder = pattern
            .iter()
            .enumerate()
            .find_map(|(pos, x)| x.data().placeholder().map(|p| (pos, p)));
        let args_delta = self.degree() as i64 - pattern.degree() as i64;
        ensure!(
            placeholder.is_some() && args_delta >= 0 || placeholder.is_none() && args_delta == 0,
            "Argument size missmatch: {} {}",
            pattern.degree(),
            self.degree()
        );

        for (p, t) in pattern.iter().zip(
            self.iter()
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
                if let Ok(mut p) = t.try_match_extend(p, r) {
                    trace!(target: "pattern_match", "New mapping: [{}]", VecDisplay(&p));
                    new_result.append(&mut p);
                }
            }
            *result = new_result;
        }

        if let Some((pos, ph)) = placeholder {
            let mapping: Vec<_> = self
                .iter()
                .enumerate()
                .filter(|(num, _)| *num >= pos && *num < 1 + pos + args_delta as usize)
                .map(|(_, x)| x.to_term())
                .collect();

            for i in result.iter_mut() {
                i.placeholders.insert(ph, mapping.clone());
            }
        }

        Ok(())
    }
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
                .join(", ")
        )
    }
}

impl<'a> fmt::Display for Subterm<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.parentheses() {
            write!(f, "(")?;
        }

        let mul_sym_str = Symbol::by_name("*")
            .map(|x| x.to_string())
            .unwrap_or("*".to_owned());

        match self.data() {
            TermNode::Symbol(symbol) => {
                let s = match symbol.display_weight() {
                    Some(_) if self.degree() < 2 => format!("{symbol}{}", self.iter().format(", ")),
                    Some(_) => self.iter().join(&symbol.to_string()),
                    // Prefix notation by default
                    None if self.degree() > 0 => format!("{symbol}({})", self.iter().format(", ")),
                    None => format!("{symbol}"),
                }
                .replace(&format!("-1{mul_sym_str}"), "-")
                .replace("+-", "-");
                write!(f, "{s}")?
            }
            TermNode::Number(num) => write!(f, "{num}")?,
            _ => write!(f, "{}", self.data())?,
        }

        if self.parentheses() {
            write!(f, ")")
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::term::{term_with_params, term_with_vars};
    use utils::VecDisplay;

    #[test]
    fn symbol_display_test() {
        for (term, display) in &[
            ("a + b + c", "a+b+c"),
            ("a*(b+c)", "a*(b+c)"),
            ("a*b + c", "a*b+c"),
            ("a*b/2 + c", "(a*b)/2+c"),
            ("a + b - c", "a+b-c"),
            ("x == -3", "x==-3"),
            ("-(-x + 2)", "-(-x+2)"),
            ("-(-1)", "--1"),
            ("118*x^2 + 1389x - 1507 == 0", "118*x^2+1389*x-1507==0"),
            // TODO: ("(-3)*(x+2)", "-3*(x+2)"),
        ] {
            let term = term_with_params(term);

            assert_eq!(term.to_string(), *display);
        }
    }

    #[test]
    fn simple_param_map_test() {
        let term = term_with_vars("x + 1 == 0");
        let pattern = term_with_params("a + b == 0");

        let maps = term
            .as_subterm()
            .try_match(pattern.as_subterm())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: 1, b: x }, { a: x, b: 1 }]");
    }

    #[test]
    fn param_mapping_minus_sign_test() {
        let term = term_with_vars("-x - 5 == 0");
        let pattern = term_with_params("-a + b == 0");

        let maps = term
            .as_subterm()
            .try_match(pattern.as_subterm())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: 5, b: -x }, { a: x, b: -5 }]");
    }

    #[test]
    fn param_mapping_minus_sign_2_test() {
        let term = term_with_vars("-x - 5 == 0");
        let pattern = term_with_params("-a - b == 0");

        let maps = term
            .as_subterm()
            .try_match(pattern.as_subterm())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: 5, b: x }, { a: x, b: 5 }]");
    }

    #[test]
    fn same_param_map_test() {
        let term = term_with_vars("x + 1 == x");
        let pattern = term_with_params("a + 1 == a");

        let maps = term
            .as_subterm()
            .try_match(pattern.as_subterm())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: x }]");
    }

    #[test]
    fn subtree_param_map_test() {
        let term = term_with_vars("2*x^2 + 4 == x - 1");
        let pattern = term_with_params("a + 4 == b");

        let maps = term
            .as_subterm()
            .try_match(pattern.as_subterm())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: 2*x^2, b: x-1 }]");
    }

    #[test]
    fn apply_param_map_test() {
        let term = term_with_vars("2*x^2 + 4 == x - 1");
        let pattern = term_with_params("a + 4 == b");

        let maps = term
            .as_subterm()
            .try_match(pattern.as_subterm())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        assert_eq!(maps.len(), 1);

        let mut test = term_with_params("a + 1");
        test.as_subterm_mut().apply_param_map(&maps[0]);
        insta::assert_debug_snapshot!(test, @"2*x^2+1");
    }

    #[test]
    fn placeholder_test() {
        let pattern = term_with_params("set(a, ..) is known");
        let term = term_with_vars("set(3, 5, 7) is known");

        let maps = term
            .as_subterm()
            .try_match(pattern.as_subterm())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(VecDisplay(&maps), @"[{ a: 3, ..1: [5, 7] }]");
    }

    #[test]
    fn placeholder_false_test() {
        let pattern = term_with_params("set(a, ..) is known");
        let term = term_with_vars("set(3) is known");

        assert!(term.as_subterm().try_match(pattern.as_subterm()).is_err());
    }
}

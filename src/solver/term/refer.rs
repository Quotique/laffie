use std::{
    collections::{HashSet, VecDeque},
    fmt,
    iter::Iterator,
};

use bigdecimal::BigDecimal;
use derive_more::{Debug, From};
use eyre::{Result, bail, ensure};
use itertools::Itertools;
use num::Zero;
use trees::Node;

use utils::SubsetIterator;

use super::{Atom, ParamSubstitution, Symbol, TermBuf, TermPath, Truth, try_sym};

pub trait Term {
    type RefType: Term;

    fn as_ref(&self) -> Self::RefType;

    fn parent(&self) -> Option<Self::RefType>;
    fn first_arg(&self) -> Option<Self::RefType>;
    fn last_arg(&self) -> Option<Self::RefType>;

    fn args_iter(&self) -> impl Iterator<Item = Self::RefType>;

    fn symbols(&self) -> HashSet<Symbol>;

    fn data(&self) -> &Atom;

    fn degree(&self) -> usize {
        self.as_ref().degree()
    }

    fn truth(&self) -> Truth;
}

#[derive(Clone, Copy)]
#[derive(PartialEq, Eq)]
#[derive(Debug, From)]
pub struct TermRef<'a>(&'a Node<Atom>);

impl<'a> Term for TermRef<'a> {
    type RefType = Self;

    fn as_ref(&self) -> Self {
        *self
    }

    fn parent(&self) -> Option<Self> {
        self.0.parent().map(Self)
    }

    fn first_arg(&self) -> Option<Self::RefType> {
        self.0.front().map(TermRef)
    }

    fn last_arg(&self) -> Option<Self::RefType> {
        self.0.back().map(TermRef)
    }

    fn args_iter(&self) -> impl Iterator<Item = Self> {
        self.0.iter().map(Self)
    }

    fn symbols(&self) -> HashSet<Symbol> {
        self.0.bfs().iter.filter_map(|x| x.data.symbol()).collect()
    }

    fn data(&self) -> &Atom {
        self.0.data()
    }

    fn degree(&self) -> usize {
        self.0.degree()
    }

    fn truth(&self) -> Truth {
        self.data()
            .symbol()
            .map(|x| x.check_truth(*self))
            .unwrap_or(Truth::Unknown)
    }
}

impl<'a> TermRef<'a> {
    #[inline]
    pub fn id(&self) -> TermPath {
        let mut current = *self;
        let mut path = vec![];

        while let Some(parent) = current.parent() {
            let id = parent
                .args_iter()
                .find_position(|x| std::ptr::eq(x.0, current.0))
                .map(|(num, _)| num)
                .expect("the parent must contains the child");
            path.push(id);
            current = parent;
        }
        path.reverse();
        path.into()
    }

    #[inline]
    pub fn bfs(&self) -> impl Iterator<Item = trees::bfs::Visit<&Atom>> {
        self.0.bfs().iter
    }

    #[inline]
    pub fn to_owned(&self) -> TermBuf {
        self.0.deep_clone().into()
    }

    pub fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = TermBuf> + '_> {
        Box::new(SubsetIterator::new(self.degree(), count).map(move |i| {
            let s = self.data().symbol().unwrap();
            let mut parts = vec![TermBuf::from(Atom::Symbol(s.clone())); count];
            for (id, child) in self.args_iter().enumerate() {
                parts[i.as_vec()[id]]
                    .term_mut()
                    .push_last_arg(child.to_owned());
            }

            for p in parts.iter_mut() {
                if p.term().degree() == 1 {
                    let mut child = p.term_mut().pop_first_arg().unwrap();
                    p.term_mut().swap(&mut child.term_mut());
                }
            }
            let mut result = TermBuf::from(Atom::Symbol(s.clone()));
            for p in parts.into_iter() {
                result.term_mut().push_last_arg(p);
            }
            result
        }))
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

    pub fn contains(&self, target: &TermRef) -> bool {
        if *self == *target {
            return true;
        }

        for child in self.args_iter() {
            if child.contains(target) {
                return true;
            }
        }
        false
    }
}

impl<'a> TermRef<'a> {
    #[inline]
    pub fn try_match(&self, pattern: TermRef) -> Result<Vec<ParamSubstitution>> {
        self.try_match_extend(pattern, Default::default())
    }

    pub fn find_matching_subterms(
        &self,
        pattern: TermRef,
    ) -> Vec<(Vec<ParamSubstitution>, TermPath)> {
        self.find_matching_subterms_extend(pattern, Default::default())
    }

    pub fn find_matching_subterms_extend(
        &self,
        pattern: TermRef,
        params: ParamSubstitution,
    ) -> Vec<(Vec<ParamSubstitution>, TermPath)> {
        let mut result = vec![];
        let mut queue = VecDeque::new();
        queue.push_back((*self, self.id()));

        while let Some((node, pos)) = queue.pop_front() {
            if let Ok(mapping) = node
                .try_match_extend(pattern, params.clone())
                .map_err(|_| trace!(target: "pattern_match", "No match for {pattern} to {node}"))
            {
                result.push((mapping, pos));
            }

            for i in node.args_iter() {
                queue.push_back((i, i.id()));
            }
        }
        result
    }

    pub fn try_match_extend(
        &self,
        pattern: TermRef,
        mut params: ParamSubstitution,
    ) -> Result<Vec<ParamSubstitution>> {
        trace!(target: "pattern_match", "Pattern: {pattern}, traget: {self}, mapping: {params}");

        match (&pattern.data(), &self.data()) {
            (Atom::Symbol(sym), Atom::Symbol(t_sym)) => {
                ensure!(sym == t_sym, "Expect symbol {sym}, found: {t_sym}");
                let mut result = vec![];

                if sym.is_associative() && sym.is_commutative() {
                    // TODO: priority mapping
                    // not even trying to map subsets twice
                    for (num, parts) in self.subsets(pattern.degree()).enumerate() {
                        ensure!(num < 1025, "Subsets of operation is too large");

                        let mut loc_result = vec![params.clone()];
                        parts
                            .term()
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
            (Atom::Symbol(mul), Atom::Number(neg)) if mul == "*" && neg < &BigDecimal::zero() => {
                TermBuf::symbol("*")
                    .arg(TermBuf::number(-1))
                    .arg(TermBuf::number(neg.abs()))
                    .term()
                    .try_match_extend(pattern, params)
            }
            (Atom::Symbol(p_id), _) => {
                bail!("Expect symbol id: {p_id}, found target: {:?}", &self.data())
            }
            (Atom::Param(p), _) => {
                if params.params.contains_key(p) {
                    let node = params.params.get(p).unwrap();
                    let _ = self.try_match(node.term())?;
                } else {
                    params.params.insert(p.clone(), self.to_owned());
                }
                Ok(vec![params])
            }
            (Atom::Number(value), Atom::Number(other_value)) if value == other_value => {
                Ok(vec![params])
            }
            (Atom::Number(value), Atom::Number(other_value)) => {
                bail!("Expect Number {value}, found {other_value}",)
            }
            (Atom::Number(_), _) => bail!("Expect Number, found: {:?}", self.data()),
            (Atom::Variable(value), Atom::Variable(other_value)) if value == other_value => {
                Ok(vec![params])
            }
            (Atom::Variable(value), Atom::Variable(other_value)) => {
                bail!("Expect Varible {value}, found {other_value}")
            }
            (Atom::Variable(_), _) => bail!("Expect Varible, found: {:?}", self.data()),
            (Atom::ArgList(_), _) => bail!("Mapping placeholder"),
        }
    }

    fn try_match_args(&self, pattern: TermRef, result: &mut Vec<ParamSubstitution>) -> Result<()> {
        let placeholder = pattern
            .args_iter()
            .enumerate()
            .find_map(|(pos, x)| x.data().placeholder().map(|p| (pos, p)));
        let args_delta = self.degree() as i64 - pattern.degree() as i64;
        ensure!(
            placeholder.is_some() && args_delta >= 0 || placeholder.is_none() && args_delta == 0,
            "Argument size missmatch: {} {}",
            pattern.degree(),
            self.degree()
        );

        for (p, t) in pattern.args_iter().zip(
            self.args_iter()
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
                    trace!(target: "pattern_match", "New mapping: [{}]", p.iter().format(", "));
                    new_result.append(&mut p);
                }
            }
            *result = new_result;
        }

        if let Some((pos, ph)) = placeholder {
            let mapping: Vec<_> = self
                .args_iter()
                .enumerate()
                .filter(|(num, _)| *num >= pos && *num < 1 + pos + args_delta as usize)
                .map(|(_, x)| x.to_owned())
                .collect();

            for i in result.iter_mut() {
                i.arglists.insert(ph, mapping.clone());
            }
        }

        Ok(())
    }
}

impl<'a> fmt::Display for TermRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.parentheses() {
            write!(f, "(")?;
        }

        let mul_sym_str = try_sym("*")
            .map(|x| x.to_string())
            .unwrap_or("*".to_owned());

        match self.data() {
            Atom::Symbol(symbol) => {
                let s = match symbol.display_weight() {
                    Some(_) if self.degree() < 2 => {
                        format!("{symbol}{}", self.args_iter().format(", "))
                    }
                    Some(_) => self.args_iter().join(&symbol.to_string()),
                    // Prefix notation by default
                    None if self.degree() > 0 => {
                        format!("{symbol}({})", self.args_iter().format(", "))
                    }
                    None => format!("{symbol}"),
                }
                .replace(&format!("-1{mul_sym_str}"), "-")
                .replace("+-", "-");
                write!(f, "{s}")?
            }
            Atom::Number(num) => write!(f, "{num}")?,
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
    use itertools::Itertools;

    use crate::term::{term_with_params, term_with_vars};

    #[test]
    fn subterm_contains_direct_term() {
        let term = term_with_vars("x");
        assert!(term.term().contains(&term.term()));
    }

    #[test]
    fn subterm_does_not_contain_absent_term() {
        let term = term_with_vars("set(1, 2, 3) is known");
        let missing = term_with_vars("4");
        assert!(!term.term().contains(&missing.term()));
    }

    #[test]
    fn subterm_contains_in_multi_arity() {
        let term = term_with_vars("set(3, 5, 7) is known");
        let target = term_with_vars("5");
        assert!(term.term().contains(&target.term()));
    }

    #[test]
    fn subterm_contains_complex_term() {
        let term = term_with_vars("a*x^2 + b*x + c == 0");
        let subterm = term_with_vars("a*x^2");
        assert!(term.term().contains(&subterm.term()));
    }

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
            ("(-3)*(x+2)", "-3*(x+2)"),
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
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: 1, b: x }, { a: x, b: 1 }");
    }

    #[test]
    fn param_mapping_minus_sign_test() {
        let term = term_with_vars("-x - 5 == 0");
        let pattern = term_with_params("-a + b == 0");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: 5, b: -x }, { a: x, b: -5 }");
    }

    #[test]
    fn param_mapping_minus_sign_2_test() {
        let term = term_with_vars("-x - 5 == 0");
        let pattern = term_with_params("-a - b == 0");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: 5, b: x }, { a: x, b: 5 }");
    }

    #[test]
    fn same_param_map_test() {
        let term = term_with_vars("x + 1 == x");
        let pattern = term_with_params("a + 1 == a");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: x }");
    }

    #[test]
    fn subtree_param_map_test() {
        let term = term_with_vars("2*x^2 + 4 == x - 1");
        let pattern = term_with_params("a + 4 == b");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: 2*x^2, b: x-1 }");
    }

    #[test]
    fn apply_param_map_test() {
        let term = term_with_vars("2*x^2 + 4 == x - 1");
        let pattern = term_with_params("a + 4 == b");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        assert_eq!(maps.len(), 1);

        let mut test = term_with_params("a + 1");
        test.term_mut().apply_param_map(&maps[0]);
        insta::assert_snapshot!(test, @"2*x^2+1");
    }

    #[test]
    fn placeholder_test() {
        let pattern = term_with_params("set(a, ..) is known");
        let term = term_with_vars("set(3, 5, 7) is known");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: 3, ..1: [5, 7] }");
    }

    #[test]
    fn placeholder_false_test() {
        let pattern = term_with_params("set(a, ..) is known");
        let term = term_with_vars("set(3) is known");

        assert!(term.term().try_match(pattern.term()).is_err());
    }
}

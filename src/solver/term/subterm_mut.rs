use std::{
    collections::{HashMap, HashSet},
    iter::Iterator,
    sync::Arc,
};

use derive_more::{Debug, Display, From};
use trees::Node;

use utils::SubsetIterator;

use super::{FuncSymbol, ParamsMapping, Subterm, Symbol, Term, TruthResult, Variable};
use crate::NormalizationLevel;

pub type VariablesMap = HashMap<Variable, Term>;

#[derive(Debug, Display, From)]
pub struct SubtermMut<'a>(&'a mut Node<Symbol>);

impl<'a> SubtermMut<'a> {
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Subterm> {
        self.0.iter().map(Subterm::from)
    }

    #[inline]
    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = SubtermMut<'b>> + 'b {
        self.0.iter_mut().map(|x| SubtermMut(x.get_mut()))
    }

    #[inline]
    pub fn first_arg(&self) -> Option<Subterm> {
        self.0.front().map(Subterm::from)
    }

    #[inline]
    pub fn last_arg(&self) -> Option<Subterm> {
        self.0.back().map(Subterm::from)
    }

    #[inline]
    pub fn first_arg_mut<'b>(&'b mut self) -> Option<SubtermMut<'b>> {
        self.0.front_mut().map(|x| SubtermMut(x.get_mut()))
    }

    #[inline]
    pub fn last_arg_mut<'b>(&'b mut self) -> Option<SubtermMut<'b>> {
        self.0.back_mut().map(|x| SubtermMut(x.get_mut()))
    }

    #[inline]
    pub fn insert_after(&mut self, term: Term) {
        self.0.insert_next_sib(term.tree);
    }

    #[inline]
    pub fn insert_before(&mut self, term: Term) {
        self.0.insert_prev_sib(term.tree);
    }

    #[inline]
    pub fn pop_first_arg(&mut self) -> Option<Term> {
        self.0.pop_front().map(Term::from)
    }

    #[inline]
    pub fn pop_last_arg(&mut self) -> Option<Term> {
        self.0.pop_back().map(Term::from)
    }

    #[inline]
    pub fn push_first_arg(&mut self, term: Term) -> &mut Self {
        self.0.push_front(term.tree);
        self
    }

    #[inline]
    pub fn push_last_arg(&mut self, term: Term) -> &mut Self {
        self.0.push_back(term.tree);
        self
    }
}

impl<'a> SubtermMut<'a> {
    #[allow(clippy::mutable_key_type)]
    pub fn symbols(&self) -> HashSet<Arc<FuncSymbol>> {
        self.0
            .bfs()
            .iter
            .filter_map(|x| x.data.func_symbol())
            .collect()
    }

    #[inline]
    pub fn to_term(&self) -> Term {
        self.0.deep_clone().into()
    }

    #[inline]
    pub fn detach(&mut self) -> Term {
        self.0.detach().into()
    }

    #[inline]
    pub fn degree(&self) -> usize {
        self.0.degree()
    }

    #[inline]
    pub fn data(&self) -> &Symbol {
        self.0.data()
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut Symbol {
        self.0.data_mut()
    }
}

impl<'a> SubtermMut<'a> {
    pub fn apply_param_map(&mut self, params: &ParamsMapping) -> &mut Self {
        match self.data().clone() {
            Symbol::Param(p) => {
                if let Some(p) = params.params.get(&p) {
                    self.swap(&mut p.clone().as_subterm_mut());
                }
            }
            Symbol::Placeholder(p) => {
                if let Some(p) = params.placeholders.get(&p) {
                    let mut p = p.clone();
                    self.swap(&mut p[0].as_subterm_mut());
                    for i in p.into_iter().skip(1).rev() {
                        self.insert_after(i);
                    }
                }
            }
            _ => {
                for mut i in self.iter_mut() {
                    i.apply_param_map(params);
                }
            }
        }
        self
    }

    pub fn apply_variable_map(&mut self, variables: &VariablesMap) {
        if let Some(mut v) = self
            .0
            .data()
            .variable()
            .and_then(|x| variables.get(x))
            .cloned()
        {
            self.swap(&mut v.as_subterm_mut());
        } else {
            for mut i in self.iter_mut() {
                i.apply_variable_map(variables);
            }
        }
    }

    pub fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = Term> + '_> {
        Box::new(SubsetIterator::new(self.0.degree(), count).map(move |i| {
            let s = self.0.data().func_symbol().unwrap();
            let mut parts = vec![Term::from(Symbol::FuncSymbol(s.clone())); count];
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
            let mut result = Term::from(Symbol::FuncSymbol(s.clone()));
            for p in parts.into_iter() {
                result.as_subterm_mut().push_last_arg(p);
            }
            result
        }))
    }

    #[inline]
    pub fn evaluate(&mut self, level: NormalizationLevel) -> bool {
        self.0
            .data()
            .func_symbol()
            .map(|x| x.evaluate(self, level))
            .unwrap_or(false)
    }

    #[inline]
    pub fn truth(&self) -> TruthResult {
        Subterm::from(self.0 as &Node<_>).truth()
    }

    pub fn swap(&mut self, other: &mut SubtermMut) {
        std::mem::swap(self.data_mut(), other.data_mut());
        let self_degree = self.degree();

        while let Some(t) = self.0.pop_front() {
            other.0.push_back(t);
        }
        while other.degree() > self_degree {
            if let Some(t) = other.pop_first_arg() {
                self.push_last_arg(t);
            }
        }
    }

    pub fn replace(&mut self, src: Subterm, dst: Subterm) {
        self.iter_mut().for_each(|mut child| {
            child.replace(src, dst);
        });
        self.iter_mut().for_each(|mut child| {
            if child == src {
                child.swap(&mut dst.to_term().as_subterm_mut());
            }
        });
    }
}

impl<'a, 'b> PartialEq<Subterm<'a>> for SubtermMut<'b> {
    fn eq(&self, other: &Subterm) -> bool {
        Subterm::from(self.0 as &Node<_>).eq(other)
    }
}

#[cfg(test)]
mod tests {
    use crate::term::term_with_params;

    #[test]
    fn replace_test() {
        let mut test_state = term_with_params("a/(a+b)");
        let test_pattern = term_with_params("(a+b)");
        let test_replace = term_with_params("c");

        test_state
            .as_subterm_mut()
            .replace(test_pattern.as_subterm(), test_replace.as_subterm());
        assert_eq!(test_state, term_with_params("a/c"));
    }
}

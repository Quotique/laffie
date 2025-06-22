use std::{
    collections::{HashMap, HashSet},
    iter::Iterator,
    sync::Arc,
};

use derive_more::{Debug, Display, From};
use trees::{Node, Tree};

use utils::SubsetIterator;

use crate::{
    symbol::{Param, Symbol, TruthResult, Variable},
    term::Term,
    NormalizationLevel,
};

use super::FuncSymbol;

pub type ParamsMap = HashMap<Param, Term>;
pub type VariablesMap = HashMap<Variable, Term>;

#[derive(Clone, Copy)]
#[derive(PartialEq, Eq)]
#[derive(Debug, Display, From)]
pub struct SymbolNode<'a>(&'a Node<Symbol>);

#[derive(Debug, Display, From)]
pub struct SymbolNodeMut<'a>(&'a mut Node<Symbol>);

pub type SymbolTree = Tree<Symbol>;

impl<'a> SymbolNode<'a> {
    pub fn degree(&self) -> usize {
        self.0.degree()
    }

    pub fn data(&self) -> &Symbol {
        self.0.data()
    }

    pub fn iter(&self) -> impl Iterator<Item = Self> {
        self.0.iter().map(Self)
    }

    pub fn front(&self) -> Option<Self> {
        self.0.front().map(SymbolNode)
    }

    pub fn back(&self) -> Option<Self> {
        self.0.back().map(SymbolNode)
    }

    pub fn deep_clone(&self) -> Term {
        self.0.deep_clone().into()
    }

    #[allow(clippy::mutable_key_type)]
    pub fn symbols(&self) -> HashSet<Arc<FuncSymbol>> {
        self.0
            .bfs()
            .iter
            .filter_map(|x| x.data.func_symbol())
            .collect()
    }

    pub fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = Term> + '_> {
        Box::new(SubsetIterator::new(self.0.degree(), count).map(move |i| {
            let s = self.0.data().func_symbol().unwrap();
            let mut parts = vec![Term::from(Symbol::FuncSymbol(s.clone())); count];
            for (id, child) in self.iter().enumerate() {
                parts[i.as_vec()[id]]
                    .root_mut()
                    .push_back(child.deep_clone());
            }

            for p in parts.iter_mut() {
                if p.root().degree() == 1 {
                    let mut child = p.root_mut().pop_front().unwrap();
                    swap_node(&mut p.root_mut(), &mut child.root_mut());
                }
            }
            let mut result = Term::from(Symbol::FuncSymbol(s.clone()));
            for p in parts.into_iter() {
                result.root_mut().push_back(p);
            }
            result
        }))
    }

    pub fn check_truth(&self) -> TruthResult {
        self.0
            .data()
            .func_symbol()
            .map(|x| x.check_truth(*self))
            .unwrap_or(TruthResult::Unknown)
    }
}

impl<'a> SymbolNodeMut<'a> {
    pub fn detach(&mut self) -> Term {
        self.0.detach().into()
    }

    pub fn degree(&self) -> usize {
        self.0.degree()
    }

    pub fn data(&self) -> &Symbol {
        self.0.data()
    }

    pub fn data_mut(&mut self) -> &mut Symbol {
        self.0.data_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = SymbolNode> {
        self.0.iter().map(SymbolNode)
    }

    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = SymbolNodeMut<'b>> + 'b {
        self.0.iter_mut().map(|x| SymbolNodeMut(x.get_mut()))
    }

    pub fn front(&self) -> Option<SymbolNode> {
        self.0.front().map(SymbolNode)
    }

    pub fn back(&self) -> Option<SymbolNode> {
        self.0.back().map(SymbolNode)
    }

    pub fn front_mut<'b>(&'b mut self) -> Option<SymbolNodeMut<'b>> {
        self.0.front_mut().map(|x| SymbolNodeMut(x.get_mut()))
    }

    pub fn back_mut<'b>(&'b mut self) -> Option<SymbolNodeMut<'b>> {
        self.0.back_mut().map(|x| SymbolNodeMut(x.get_mut()))
    }

    pub fn insert_after(&mut self, term: Term) {
        self.0.insert_next_sib(term.tree);
    }

    pub fn insert_before(&mut self, term: Term) {
        self.0.insert_prev_sib(term.tree);
    }

    pub fn pop_front(&mut self) -> Option<Term> {
        self.0.pop_front().map(Term::from)
    }

    pub fn pop_back(&mut self) -> Option<Term> {
        self.0.pop_back().map(Term::from)
    }

    pub fn push_front(&mut self, term: Term) -> &mut Self {
        self.0.push_front(term.tree);
        self
    }

    pub fn push_back(&mut self, term: Term) -> &mut Self {
        self.0.push_back(term.tree);
        self
    }

    pub fn deep_clone(&self) -> Term {
        self.0.deep_clone().into()
    }

    #[allow(clippy::mutable_key_type)]
    pub fn symbols(&self) -> HashSet<Arc<FuncSymbol>> {
        self.0
            .bfs()
            .iter
            .filter_map(|x| x.data.func_symbol())
            .collect()
    }

    pub fn apply_variable_map(&mut self, variables: &VariablesMap) {
        if let Some(mut v) = self
            .0
            .data()
            .variable()
            .and_then(|x| variables.get(x))
            .cloned()
        {
            swap_node(self, &mut v.root_mut());
        } else {
            for mut i in self.iter_mut() {
                i.apply_variable_map(variables);
            }
        }
    }

    pub fn apply_param_map(&mut self, params: &ParamsMap) {
        if let Some(mut v) = self.0.data().param().and_then(|x| params.get(x)).cloned() {
            swap_node(self, &mut v.root_mut());
        } else {
            for mut i in self.iter_mut() {
                i.apply_param_map(params);
            }
        }
    }

    pub fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = Term> + '_> {
        Box::new(SubsetIterator::new(self.0.degree(), count).map(move |i| {
            let s = self.0.data().func_symbol().unwrap();
            let mut parts = vec![Term::from(Symbol::FuncSymbol(s.clone())); count];
            for (id, child) in self.iter().enumerate() {
                parts[i.as_vec()[id]]
                    .root_mut()
                    .push_back(child.deep_clone());
            }

            for p in parts.iter_mut() {
                if p.root().degree() == 1 {
                    let mut child = p.root_mut().pop_front().unwrap();
                    swap_node(&mut p.root_mut(), &mut child.root_mut());
                }
            }
            let mut result = Term::from(Symbol::FuncSymbol(s.clone()));
            for p in parts.into_iter() {
                result.root_mut().push_back(p);
            }
            result
        }))
    }

    pub fn evaluate(&mut self, level: NormalizationLevel) -> bool {
        self.0
            .data()
            .func_symbol()
            .map(|x| x.evaluate(self, level))
            .unwrap_or(false)
    }

    pub fn check_truth(&self) -> TruthResult {
        let node = SymbolNode(self.0);
        node.check_truth()
    }
}

impl<'a, 'b> PartialEq<SymbolNode<'a>> for SymbolNodeMut<'b> {
    fn eq(&self, other: &SymbolNode) -> bool {
        (*self.0).eq(other.0)
    }
}

pub fn swap_node(l: &mut SymbolNodeMut, r: &mut SymbolNodeMut) {
    let mut l_childs: Vec<Term> = vec![];
    let mut r_childs: Vec<Term> = vec![];

    while let Some(t) = l.pop_front() {
        l_childs.push(t);
    }
    while let Some(t) = r.pop_front() {
        r_childs.push(t);
    }

    let r_data = r.data().clone();
    *r.data_mut() = l.data().clone();
    *l.data_mut() = r_data;
    //(l.root_mut().data, r.root_mut().data) = (r.root().data, l.root().data);
    for i in l_childs {
        r.push_back(i);
    }
    for i in r_childs {
        l.push_back(i);
    }
}

pub fn replace(arg: &mut SymbolNodeMut, src: SymbolNode, dst: SymbolNode) {
    arg.iter_mut().for_each(|mut child| {
        replace(&mut child, src, dst);
    });
    arg.iter_mut().for_each(|mut child| {
        if child == src {
            swap_node(&mut child, &mut dst.deep_clone().root_mut());
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::term::term_with_params;

    use super::*;

    #[test]
    fn replace_test() {
        let mut test_state = term_with_params("a/(a+b)");
        let test_pattern = term_with_params("(a+b)");
        let test_replace = term_with_params("c");

        replace(
            &mut test_state.root_mut(),
            test_pattern.root(),
            test_replace.root(),
        );
        assert_eq!(test_state, term_with_params("a/c"));
    }
}

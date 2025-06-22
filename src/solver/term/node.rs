use std::{collections::HashSet, iter::Iterator, sync::Arc};

use derive_more::{Debug, Display, From};
use trees::Node;

use utils::SubsetIterator;

use crate::term::{Symbol, Term, TruthResult};

use super::{swap_node, FuncSymbol};

#[derive(Clone, Copy)]
#[derive(PartialEq, Eq)]
#[derive(Debug, Display, From)]
pub struct SymbolNode<'a>(&'a Node<Symbol>);

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

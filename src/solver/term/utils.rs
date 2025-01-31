use std::{
    collections::{HashMap, HashSet},
    iter::Iterator,
    ops::Deref,
    sync::Arc,
};

use trees::{tr, Node, Tree};

use utils::SubsetIterator;

use crate::{
    symbol::{Param, Symbol, TruthResult, Variable},
    NormalizationLevel,
};

use super::{FuncSymbol, SymbolNode, SymbolTree};

pub type ParamsMap = HashMap<Param, SymbolTree>;
pub type VariablesMap = HashMap<Variable, SymbolTree>;

pub fn swap_node<F: Clone>(l: &mut Node<F>, r: &mut Node<F>) {
    let mut l_childs: Vec<Tree<F>> = vec![];
    let mut r_childs: Vec<Tree<F>> = vec![];

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

pub fn replace<F: Clone + PartialEq + Unpin>(arg: &mut Node<F>, src: &Node<F>, dst: &Node<F>) {
    arg.iter_mut().for_each(|child| {
        replace(child.get_mut(), src, dst);
    });
    arg.iter_mut().for_each(|child| {
        if child.deref() == src {
            swap_node(child.get_mut(), &mut dst.deep_clone().root_mut());
        }
    });
}

pub trait NodeMapping {
    fn apply_variable_map(&mut self, variables: &VariablesMap);

    fn apply_param_map(&mut self, params: &ParamsMap);

    fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = SymbolTree> + '_>;

    fn evaluate(&mut self, level: NormalizationLevel) -> bool;

    fn check_truth(&self) -> TruthResult;

    #[allow(clippy::mutable_key_type)]
    fn symbols(&self) -> HashSet<Arc<FuncSymbol>>;
}

impl NodeMapping for SymbolNode {
    fn symbols(&self) -> HashSet<Arc<FuncSymbol>> {
        self.bfs()
            .iter
            .filter_map(|x| x.data.func_symbol())
            .collect()
    }

    fn apply_variable_map(&mut self, variables: &VariablesMap) {
        if let Some(mut v) = self
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

    fn apply_param_map(&mut self, params: &ParamsMap) {
        if let Some(mut v) = self.data().param().and_then(|x| params.get(x)).cloned() {
            swap_node(self, &mut v.root_mut());
        } else {
            for mut i in self.iter_mut() {
                i.apply_param_map(params);
            }
        }
    }

    fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = SymbolTree> + '_> {
        Box::new(SubsetIterator::new(self.degree(), count).map(move |i| {
            let s = self.data().func_symbol().unwrap();
            let mut parts = vec![tr(Symbol::FuncSymbol(s.clone())); count];
            for (id, child) in self.iter().enumerate() {
                parts[i.as_vec()[id]].push_back(child.deep_clone());
            }

            for p in parts.iter_mut() {
                if p.degree() == 1 {
                    let mut child = p.pop_front().unwrap();
                    swap_node(&mut p.root_mut(), &mut child.root_mut());
                }
            }
            let mut result = tr(Symbol::FuncSymbol(s.clone()));
            for p in parts.into_iter() {
                result.push_back(p);
            }
            result
        }))
    }

    fn evaluate(&mut self, level: NormalizationLevel) -> bool {
        self.data()
            .func_symbol()
            .map(|x| x.evaluate(self, level))
            .unwrap_or(false)
    }

    fn check_truth(&self) -> TruthResult {
        self.data()
            .func_symbol()
            .map(|x| x.check_truth(self))
            .unwrap_or(TruthResult::Unknown)
    }
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
            test_state.root_mut().get_mut(),
            test_pattern.root(),
            test_replace.root(),
        );
        assert_eq!(test_state, term_with_params("a/c"));
    }
}

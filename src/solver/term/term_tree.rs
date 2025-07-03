use std::{
    collections::{HashMap, HashSet},
    convert::From,
    fmt,
};

use derive_more::From;
use indexmap::IndexMap;
use trees::Tree;

use super::{ArgList, Param, Subterm, SubtermMut, Symbol, TermNode, Variable};
use crate::{CompactString, Decimal, NormalizationLevel};

type SymbolTree = Tree<TermNode>;

pub type VariablesMap = HashMap<Variable, Term>;

#[derive(Debug, Default, Clone, From, PartialEq, Eq, Hash)]
pub struct SubtermId(pub(super) Vec<usize>);

#[derive(Debug, Clone, Default)]
pub struct ParamsMapping {
    pub params:   IndexMap<Param, Term>,
    pub arglists: IndexMap<ArgList, Vec<Term>>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Term(SymbolTree);

impl Term {
    #[inline]
    pub fn symbol(symbol: impl AsRef<str>) -> Self {
        Self::from(TermNode::with_symbol(symbol.as_ref()))
    }

    #[inline]
    pub fn number(num: impl Into<Decimal>) -> Self {
        Self::from(TermNode::Number(num.into()))
    }

    #[inline]
    pub fn variable(var: impl Into<CompactString>) -> Self {
        Self::from(TermNode::Variable(var.into().into()))
    }

    #[inline]
    pub fn param(param: impl Into<CompactString>) -> Self {
        Self::from(TermNode::Param(param.into().into()))
    }

    #[inline]
    pub fn with_child(mut self, child: Self) -> Self {
        self.as_subterm_mut().push_last_arg(child);
        self
    }

    #[inline]
    pub fn one() -> Self {
        Self::number(1)
    }

    #[inline]
    pub fn zero() -> Self {
        Self::number(0)
    }
}

impl Term {
    #[inline]
    pub fn normalize(mut self, level: NormalizationLevel) -> Self {
        self.as_subterm_mut().normalize(level);
        self
    }

    pub fn symbols(&self) -> HashSet<Symbol> {
        self.0
            .root()
            .bfs()
            .iter
            .filter_map(|x| x.data.symbol())
            .collect()
    }

    #[inline]
    pub fn replace(&mut self, src: &Self, dst: &Self) {
        self.as_subterm_mut()
            .replace(src.as_subterm(), dst.as_subterm())
    }

    #[inline]
    pub fn data(&self) -> &TermNode {
        self.0.root().data()
    }

    #[inline]
    pub fn get(&self, id: &SubtermId) -> Option<Subterm> {
        let mut root = self.0.root();
        for i in id.0.iter() {
            root = root.iter().nth(*i)?;
        }
        Some(root.into())
    }

    #[inline]
    pub fn get_mut(&mut self, id: &SubtermId) -> Option<SubtermMut> {
        let mut root = self.0.root_mut().get_mut();
        for i in id.0.iter() {
            let next_root = root.iter_mut().nth(*i)?.get_mut();
            root = next_root;
        }

        Some(root.into())
    }

    #[inline]
    pub fn as_subterm(&self) -> Subterm {
        self.0.root().into()
    }

    #[inline]
    pub fn as_subterm_mut(&mut self) -> SubtermMut {
        self.0.root_mut().get_mut().into()
    }

    pub fn apply_map(&self, params: &ParamsMapping) -> Self {
        let mut result = self.clone();
        result.as_subterm_mut().apply_param_map(params);
        result
    }

    #[inline]
    pub fn swap(&mut self, node: &mut SubtermMut) {
        self.as_subterm_mut().swap(node);
    }
}

impl From<Term> for SymbolTree {
    fn from(value: Term) -> Self {
        value.0
    }
}

impl FromIterator<(Param, Term)> for ParamsMapping {
    fn from_iter<I: IntoIterator<Item = (Param, Term)>>(iter: I) -> Self {
        ParamsMapping {
            params:   FromIterator::from_iter(iter),
            arglists: Default::default(),
        }
    }
}

impl From<TermNode> for Term {
    fn from(value: TermNode) -> Self {
        Self::from(Tree::new(value))
    }
}

impl From<SymbolTree> for Term {
    fn from(source: SymbolTree) -> Self {
        Self(source)
    }
}

impl fmt::Debug for Term {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_subterm())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        rc::Rc,
    };

    use crate::term::{term_with_params, ArgList};

    use super::*;

    #[test]
    fn unification_test() {
        let test =
            term_with_params("2*x*x + x + 3*x + 4 + 2 == 0").normalize(NormalizationLevel::max());
        let test_norm =
            term_with_params("2*x^2 + 4*x + 6 == 0").normalize(NormalizationLevel::max());
        assert_eq!(test, test_norm);
    }

    #[test]
    fn unification_with_minus_test() {
        let test =
            term_with_params("x^2 + (-5)*x - x + 5 == 0").normalize(NormalizationLevel::max());
        let test_norm =
            term_with_params("x^2 + (-6)*x + 5 == 0").normalize(NormalizationLevel::max());
        assert_eq!(test, test_norm);
    }

    #[test]
    fn placeholder_test() {
        let test = "set(a, ..) is known <=> true";

        let term = term_with_params(test);

        assert_eq!(
            term.as_subterm()
                .first_arg()
                .unwrap()
                .first_arg()
                .unwrap()
                .last_arg()
                .unwrap()
                .data()
                .placeholder(),
            Some(ArgList::from(1))
        );
    }

    #[test]
    fn hash_test() {
        let term = term_with_params("a*x + c == 0");
        let mut s = DefaultHasher::new();
        term.hash(&mut s);
        let hash_1 = s.finish();

        let term = Rc::new(term);
        let mut s = DefaultHasher::new();
        term.hash(&mut s);
        let hash_2 = s.finish();

        assert_eq!(hash_1, hash_2);
    }
}

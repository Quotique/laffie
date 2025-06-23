use std::{
    collections::{HashMap, HashSet},
    convert::From,
    fmt,
    hash::Hash,
    sync::Arc,
};

use trees::{tr, Tree};

use super::{
    normalize, FuncSymbol, NodePosition, Param, ParamsMapping, Subterm, SubtermMut, Symbol,
};
use crate::{CompactString, Decimal, NormalizationLevel};

pub type SymbolTree = Tree<Symbol>;

#[derive(Clone, Eq)]
pub struct Term {
    pub(super) tree: SymbolTree,
    // TODO: fix binds in node operations
    pub binds:       HashMap<Param, NodePosition>,
}

impl Term {
    #[inline]
    pub fn new(tree: SymbolTree, binds: HashMap<Param, NodePosition>) -> Self {
        Self { tree, binds }
    }

    #[inline]
    pub fn func(symbol: impl AsRef<str>) -> Self {
        Self::from(Symbol::with_func_symbol(symbol.as_ref()))
    }

    #[inline]
    pub fn number(num: impl Into<Decimal>) -> Self {
        Self::from(Symbol::Number(num.into()))
    }

    #[inline]
    pub fn variable(var: impl Into<CompactString>) -> Self {
        Self::from(Symbol::Variable(var.into().into()))
    }

    #[inline]
    pub fn param(param: impl Into<CompactString>) -> Self {
        Self::from(Symbol::Param(param.into().into()))
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
        normalize(&mut self.as_subterm_mut(), level);
        self
    }

    #[allow(clippy::mutable_key_type)]
    pub fn func_symbols(&self) -> HashSet<Arc<FuncSymbol>> {
        self.tree
            .root()
            .bfs()
            .iter
            .filter_map(|x| x.data.func_symbol())
            .collect()
    }

    #[inline]
    pub fn replace(&mut self, src: &Self, dst: &Self) {
        self.as_subterm_mut()
            .replace(src.as_subterm(), dst.as_subterm())
    }

    #[inline]
    pub fn data(&self) -> &Symbol {
        self.tree.data()
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut Symbol {
        self.tree.root_mut().get_mut().data_mut()
    }

    #[inline]
    pub fn as_subterm(&self) -> Subterm {
        self.tree.root().into()
    }

    #[inline]
    pub fn as_subterm_mut(&mut self) -> SubtermMut {
        self.tree.root_mut().get_mut().into()
    }

    #[inline]
    pub fn destruct(mut self) -> (SymbolTree, trees::Forest<Symbol>) {
        let childs = self.tree.abandon();
        (self.tree, childs)
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

impl Hash for Term {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tree.hash(state);
    }
}

impl PartialEq for Term {
    fn eq(&self, other: &Term) -> bool {
        self.tree.eq(&other.tree)
    }
}

impl From<Symbol> for Term {
    fn from(value: Symbol) -> Self {
        Self::from(tr(value))
    }
}

impl From<SymbolTree> for Term {
    fn from(source: SymbolTree) -> Self {
        Self {
            tree:  source,
            binds: Default::default(),
        }
    }
}

impl fmt::Debug for Term {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_subterm())
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
        str::FromStr,
    };

    use crate::term::{term_with_params, Placeholder};

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
    fn binds_test() {
        let test = "set(a, b) as S is known <=> true";

        let term = term_with_params(test);

        insta::assert_debug_snapshot!(term, @"set(a, b) is known ⟺  true");
        assert_eq!(
            term.binds.get(&Param::from_str("S").unwrap()),
            Some(NodePosition::root().child(0).child(0)).as_ref()
        );
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
            Some(Placeholder::from(1))
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

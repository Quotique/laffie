//#![warn(missing_docs)]

mod codec;
mod func_symbol;
mod index;
mod mapping;
mod symbol;
mod term_display;
mod term_props;
mod tree_utils;

use std::{
    collections::{HashMap, HashSet},
    convert::From,
    fmt,
    hash::Hash,
    sync::Arc,
};

use eyre::Result;
use trees::{tr, Node, Tree};

use crate::NormalizationLevel;

use term_display::display_string;

pub use func_symbol::{FuncSymbol, SymbolAttr, SymbolAttrValue, TruthChecker, TruthResult};
pub use index::NodePosition;
pub use mapping::ParamsMapping;
pub use symbol::{Param, Placeholder, Symbol, Variable};
pub use term_props::TermProps;
pub use tree_utils::{replace, swap_node, NodeMapping, VariablesMap};

pub type TermTree = Tree<Symbol>;
pub type TermNode = Node<Symbol>;

#[derive(Clone, Eq)]
pub struct Term {
    pub(super) tree: TermTree,
    pub binds:       HashMap<Param, NodePosition>,
}

impl Term {
    pub fn new(tree: TermTree, binds: HashMap<Param, NodePosition>) -> Self {
        Term { tree, binds }
    }

    pub fn one() -> Self {
        Self {
            tree:  tr(Symbol::Number(1.into())),
            binds: Default::default(),
        }
    }

    pub fn zero() -> Self {
        Self {
            tree:  tr(Symbol::Number(0.into())),
            binds: Default::default(),
        }
    }

    pub fn normalize(mut self, level: NormalizationLevel) -> Self {
        crate::predefine::normalize(&mut self.tree.root_mut(), level);
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

    pub fn root(&self) -> &trees::Node<Symbol> {
        self.tree.root()
    }

    pub fn replace(&mut self, src: &Self, dst: &Self) {
        replace(self.root_mut().get_mut(), src.root(), dst.root())
    }

    pub fn root_mut(&mut self) -> std::pin::Pin<&mut trees::Node<Symbol>> {
        self.tree.root_mut()
    }

    pub fn destruct(mut self) -> (TermTree, trees::Forest<Symbol>) {
        let childs = self.tree.abandon();
        (self.tree, childs)
    }

    pub fn map(&self, target: &Self) -> Result<Vec<ParamsMapping>> {
        ParamsMapping::mapper(target.tree.root(), self.tree.root()).try_map()
    }

    pub fn apply_map(&self, params: &ParamsMapping) -> Self {
        let mut result = self.clone();
        params.apply(&mut result.tree.root_mut());
        result
    }

    pub fn swap_node(&mut self, node: &mut Node<Symbol>) {
        swap_node(&mut self.tree.root_mut(), node)
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

impl From<TermTree> for Term {
    fn from(source: TermTree) -> Self {
        Self {
            tree:  source,
            binds: Default::default(),
        }
    }
}

impl fmt::Debug for Term {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", display_string(self.tree.root()))
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", display_string(self.tree.root()))
    }
}

#[cfg(test)]
pub fn term_with_params(text: &'static str) -> Term {
    let states = parser::lang::terms(text).unwrap();
    let term = parser::TermParser::new(&states[0]).parse().unwrap();

    unsafe { std::mem::transmute::<_, Term>(term) }
}

#[cfg(test)]
pub fn term_with_vars(text: &'static str) -> Term {
    let states = parser::lang::terms(text).unwrap();
    let term = parser::TermParser::new(&states[0])
        .with_variables()
        .parse()
        .unwrap();

    unsafe { std::mem::transmute::<_, Term>(term) }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        rc::Rc,
        str::FromStr,
    };

    use crate::term::{symbol::Placeholder, term_with_params};

    use super::*;

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
            term.root()
                .front()
                .unwrap()
                .front()
                .unwrap()
                .back()
                .unwrap()
                .data()
                .placeholder(),
            Some(&Placeholder::from(1))
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

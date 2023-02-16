use std::{
    collections::{HashMap, HashSet},
    convert::From,
    fmt,
    hash::Hash,
};

use eyre::Result;
use trees::{tr, Node};

use crate::SymbolId;

use super::{
    index::NodePosition,
    mapping::ParamsMapping,
    statement_display::display_string,
    term::{Param, StatementTree, Term},
    tree_utils::{replace, swap_node},
};

#[derive(Clone, Eq)]
pub struct Statement {
    pub(super) tree: StatementTree,
    pub binds:       HashMap<Param, NodePosition>,
}

impl Statement {
    pub fn new(tree: StatementTree, binds: HashMap<Param, NodePosition>) -> Self {
        Statement { tree, binds }
    }

    pub fn one() -> Self {
        Self {
            tree:  tr(Term::Number(1.into())),
            binds: Default::default(),
        }
    }

    pub fn zero() -> Self {
        Self {
            tree:  tr(Term::Number(0.into())),
            binds: Default::default(),
        }
    }

    pub fn normalize(&self) -> Self {
        let mut copy = self.clone();
        copy.inpl_normalize();
        copy
    }

    pub fn inpl_normalize(&mut self) {
        crate::predefine::normalize(&mut self.tree.root_mut());
    }

    pub fn symbols(&self) -> HashSet<SymbolId> {
        self.tree
            .root()
            .bfs()
            .iter
            .filter_map(|x| x.data.symbol_id())
            .collect()
    }

    pub fn root(&self) -> &trees::Node<Term> {
        self.tree.root()
    }

    pub fn replace(&mut self, src: &Self, dst: &Self) {
        replace(self.root_mut().get_mut(), src.root(), dst.root())
    }

    pub fn root_mut(&mut self) -> std::pin::Pin<&mut trees::Node<Term>> {
        self.tree.root_mut()
    }

    pub fn destruct(mut self) -> (StatementTree, trees::Forest<Term>) {
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

    pub fn swap_node(&mut self, node: &mut Node<Term>) {
        swap_node(&mut self.tree.root_mut(), node)
    }
}

impl Hash for Statement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tree.hash(state);
    }
}

impl PartialEq for Statement {
    fn eq(&self, other: &Statement) -> bool {
        self.tree.eq(&other.tree)
    }
}

impl From<StatementTree> for Statement {
    fn from(source: StatementTree) -> Self {
        Self {
            tree:  source,
            binds: Default::default(),
        }
    }
}

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", display_string(self.tree.root()))
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", display_string(self.tree.root()))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::statement::{statement_with_params, term::Placeholder};

    use super::*;

    #[test]
    fn binds_test() {
        let test = "set(a, b) as S is known <=> true";

        let statement = statement_with_params(test);

        insta::assert_debug_snapshot!(statement);
        assert_eq!(
            statement.binds.get(&Param::from_str("S").unwrap()),
            Some(NodePosition::root().child(0).child(0)).as_ref()
        );
    }

    #[test]
    fn placeholder_test() {
        let test = "set(a, ..) is known <=> true";

        let statement = statement_with_params(test);

        assert_eq!(
            statement
                .root()
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
}

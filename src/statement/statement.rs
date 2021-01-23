use crate::core::term::{display_string, StatementTree, Term};
use std::{
    collections::{HashMap, HashSet},
    convert::From,
    fmt,
    hash::{Hash, Hasher},
};
use trees::Node;

pub type ParamsMap = HashMap<String, u64>;

#[derive(Clone, PartialEq, Eq)]
pub struct Statement {
    tree: StatementTree,
}

impl Statement {
    pub fn normalize(&self) -> Self {
        let mut copy = self.clone();
        copy.inpl_normalize();
        copy
    }

    pub fn inpl_normalize(&mut self) {
        crate::solver::operations::normalize(self.tree.root_mut());
    }

    pub fn symbols(&self) -> HashSet<u64> {
        self.tree
            .root()
            .bfs()
            .iter
            .filter_map(|x| x.data.symbol_id())
            .collect::<HashSet<u64>>()
    }

    fn contains(term: &Term, tree: &Node<Term>) -> bool {
        if &tree.data == term {
            return true;
        }

        for i in tree.iter() {
            if Self::contains(term, i) {
                return true;
            }
        }
        false
    }
}

impl From<StatementTree> for Statement {
    fn from(source: StatementTree) -> Self {
        Self { tree: source }
    }
}

impl Hash for Statement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tree.root().hash(state);
    }
}

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", display_string(&self.tree.root()))
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", display_string(&self.tree.root()))
    }
}

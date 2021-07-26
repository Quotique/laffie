use super::{
    term::{display_string, StatementTree, Term},
    tree_utils::{apply_map, params_map, replace, swap_node},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    convert::From,
    fmt,
    hash::Hash,
    ops::{Index, IndexMut},
};
use trees::{tr, Node};

pub type ParamsMap = HashMap<String, u64>;
pub type ReverseParamsMap = HashMap<u64, StatementTree>;

#[derive(Clone, PartialEq, Eq)]
pub struct NodePosition {
    coordinates: Vec<usize>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Statement {
    tree: StatementTree,
}

impl Statement {
    pub fn one() -> Self {
        Self {
            tree: tr(Term::Number(1.into())),
        }
    }

    pub fn zero() -> Self {
        Self {
            tree: tr(Term::Number(0.into())),
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

    pub fn symbols(&self) -> HashSet<u64> {
        self.tree
            .root()
            .bfs()
            .iter
            .filter_map(|x| x.data.symbol_id())
            .collect::<HashSet<u64>>()
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

    pub fn map(&self, target: &Self) -> Result<Vec<ReverseParamsMap>, String> {
        params_map(target.tree.root(), self.tree.root())
    }

    pub fn apply_map(&self, params: &ReverseParamsMap) -> Self {
        let mut result = self.clone();
        apply_map(&mut result.tree.root_mut(), params);
        result
    }

    pub fn find_subtree_map(&self, target: &Self) -> Vec<(Vec<ReverseParamsMap>, NodePosition)> {
        let mut result = vec![];
        let mut queue = VecDeque::new();
        queue.push_back((target.tree.root(), NodePosition::root()));

        while let Some((node, pos)) = queue.pop_front() {
            if let Ok(mapping) = params_map(node, self.tree.root()).map_err(
                |_| trace!(target: "pattern_match", "No match for {} to {}", self.tree, node),
            ) {
                result.push((mapping, pos.clone()));
            }

            for (num, i) in node.iter().enumerate() {
                queue.push_back((i, pos.clone().child(num)));
            }
        }
        result
    }

    pub fn swap_node(&mut self, node: &mut Node<Term>) {
        swap_node(&mut self.tree.root_mut(), node)
    }
}

impl NodePosition {
    fn root() -> Self {
        Self {
            coordinates: vec![],
        }
    }

    fn child(mut self, num: usize) -> Self {
        self.coordinates.push(num);
        self
    }
}

impl IndexMut<&NodePosition> for Statement {
    fn index_mut(&mut self, pos: &NodePosition) -> &mut Self::Output {
        let mut root = self.tree.root_mut().get_mut();
        for i in pos.coordinates.iter() {
            let next_root = root.iter_mut().nth(*i).expect("Bad position").get_mut();
            root = next_root;
        }
        root
    }
}

impl Index<&NodePosition> for Statement {
    type Output = Node<Term>;

    fn index(&self, pos: &NodePosition) -> &Self::Output {
        let mut root = self.tree.root();
        for i in pos.coordinates.iter() {
            root = root.iter().nth(*i).expect("Bad position");
        }
        root
    }
}

impl From<StatementTree> for Statement {
    fn from(source: StatementTree) -> Self {
        Self { tree: source }
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

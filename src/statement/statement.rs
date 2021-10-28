use std::{
    collections::{HashMap, HashSet, VecDeque},
    convert::From,
    fmt,
    hash::Hash,
    ops::{Index, IndexMut},
    str::FromStr,
};

use bigdecimal::BigDecimal as Decimal;
use eyre::{ensure, Result};
use trees::{tr, Node};

use crate::parser::Node as ParserNode;

use super::{
    statement_display::display_string,
    symbols::symbol_by_name,
    term::{Param, StatementTree, Term, Variable},
    tree_utils::{replace, swap_node, NodeMapping},
};

#[derive(Clone, Copy)]
enum NodeType {
    Statement,
    Rule,
}

pub type ParamsMap = HashMap<Param, StatementTree>;

#[derive(Clone, PartialEq, Eq)]
pub struct NodePosition {
    coordinates: Vec<usize>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Statement {
    tree: StatementTree,
}

impl Statement {
    #[inline]
    pub fn try_parse_statement(node: &ParserNode) -> Result<Self> {
        Ok(Self {
            tree: Self::try_parse_impl(node, NodeType::Statement)?,
        })
    }

    #[inline]
    pub fn try_parse_rule(node: &ParserNode) -> Result<Self> {
        Ok(Self {
            tree: Self::try_parse_impl(node, NodeType::Rule)?,
        })
    }

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

    pub fn map(&self, target: &Self) -> Result<Vec<ParamsMap>> {
        target.tree.root().params_map(self.tree.root())
    }

    pub fn apply_map(&self, params: &ParamsMap) -> Self {
        let mut result = self.clone();
        result.tree.root_mut().apply_param_map(params);
        result
    }

    pub fn find_subtree_map(&self, target: &Self) -> Vec<(Vec<ParamsMap>, NodePosition)> {
        let mut result = vec![];
        let mut queue = VecDeque::new();
        queue.push_back((target.tree.root(), NodePosition::root()));

        while let Some((node, pos)) = queue.pop_front() {
            if let Ok(mapping) = node.params_map(self.tree.root()).map_err(
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

    fn try_parse_impl(node: &ParserNode, node_type: NodeType) -> Result<StatementTree> {
        let mut tree = tr(Self::parse_term(node.data().as_str(), &node_type));
        if tree.root().data().symbol_id().is_some() {
            for child in node.iter() {
                tree.push_back(Self::try_parse_impl(child, node_type)?);
            }
        } else {
            ensure!(
                node.degree() == 0,
                "Node {} can't contains children!",
                &node.data()
            );
        }

        Ok(tree)
    }

    fn parse_term(data: &str, node_type: &NodeType) -> Term {
        if let Ok(value) = Decimal::from_str(data) {
            Term::Number(value)
        } else if let Some(symbol) = symbol_by_name(data) {
            Term::Symbol(symbol.id)
        } else {
            match node_type {
                NodeType::Rule => {
                    Term::Param(Param::from_str(data).expect("unable to create param"))
                }
                NodeType::Statement => {
                    Term::Variable(Variable::from_str(data).expect("unable to create variable"))
                }
            }
        }
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

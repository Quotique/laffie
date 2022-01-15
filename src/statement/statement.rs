use std::{
    collections::{HashMap, HashSet},
    convert::From,
    fmt,
    hash::Hash,
    str::FromStr,
};

use bigdecimal::BigDecimal as Decimal;
use eyre::{ensure, Result};
use trees::{tr, Node};

use crate::parser::Node as ParserNode;

use super::{
    index::NodePosition,
    mapping::ParamsMapping,
    statement_display::display_string,
    symbols::symbol_by_name,
    term::{Param, Placeholder, StatementTree, Term, Variable},
    tree_utils::{replace, swap_node},
};

#[derive(Clone, Copy)]
enum NodeType {
    Statement,
    Rule,
}

#[derive(Clone, Eq)]
pub struct Statement {
    pub(super) tree: StatementTree,
    pub binds:       HashMap<Param, NodePosition>,
}

impl Statement {
    #[inline]
    pub fn try_parse_statement(node: &ParserNode) -> Result<Self> {
        let mut positions_map = Default::default();
        Ok(Self {
            tree:  Self::try_parse_impl(
                node,
                NodeType::Statement,
                Default::default(),
                &mut positions_map,
                &mut 0,
            )?,
            binds: positions_map,
        })
    }

    #[inline]
    pub fn try_parse_rule(node: &ParserNode) -> Result<Self> {
        let mut positions_map = Default::default();
        Ok(Self {
            tree:  Self::try_parse_impl(
                node,
                NodeType::Rule,
                Default::default(),
                &mut positions_map,
                &mut 0,
            )?,
            binds: positions_map,
        })
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

    fn try_parse_impl(
        mut node: &ParserNode,
        node_type: NodeType,
        node_position: NodePosition,
        positions_map: &mut HashMap<Param, NodePosition>,
        last_placeholder_id: &mut u64,
    ) -> Result<StatementTree> {
        while node.data() == "as" {
            let param =
                Param::from_str(node.back().unwrap().data()).expect("unable to create param");
            ensure!(
                positions_map
                    .insert(param.clone(), node_position.clone())
                    .is_none(),
                "Multiple definition of param {}",
                param
            );

            node = node.front().unwrap();
        }

        let mut tree = tr(Self::parse_term(
            node.data().as_str(),
            &node_type,
            last_placeholder_id,
        ));
        if tree.root().data().symbol_id().is_some() {
            for (num, child) in node.iter().enumerate() {
                tree.push_back(Self::try_parse_impl(
                    child,
                    node_type,
                    node_position.clone().child(num),
                    positions_map,
                    last_placeholder_id,
                )?);
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

    fn parse_term(data: &str, node_type: &NodeType, last_placeholder_id: &mut u64) -> Term {
        if data == ".." {
            *last_placeholder_id += 1;
            Term::Placeholder(Placeholder::from(*last_placeholder_id))
        } else if let Ok(value) = Decimal::from_str(data) {
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
    use super::*;
    use parser::statement_with_params;
    use predefine::setup;

    #[test]
    fn binds_test() {
        setup();

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
        setup();

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

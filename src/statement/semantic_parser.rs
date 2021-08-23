use std::str::FromStr;

use bigdecimal::BigDecimal as Decimal;
use eyre::{ensure, Result};
use trees::{tr, Node};

use super::{
    symbols::symbol_by_name,
    term::{Param, StatementTree, Term, Variable},
};

pub type ParserNode = Node<String>;

#[derive(Clone, Copy)]
pub enum NodeType {
    Statement,
    Rule,
}

pub trait TreeExtends: Sized {
    fn try_parse_statement(node: &ParserNode) -> Result<Self> {
        Self::try_parse_impl(node, NodeType::Statement)
    }
    fn try_parse_rule(node: &ParserNode) -> Result<Self> {
        Self::try_parse_impl(node, NodeType::Rule)
    }

    fn try_parse_impl(node: &ParserNode, node_type: NodeType) -> Result<Self>;
}

impl TreeExtends for StatementTree {
    fn try_parse_impl(node: &ParserNode, node_type: NodeType) -> Result<Self> {
        let mut result = tr(parse_term(node.data().as_str(), &node_type));
        if result.root().data().symbol_id().is_some() {
            for child in node.iter() {
                result.push_back(Self::try_parse_impl(child, node_type)?);
            }
        } else {
            ensure!(
                node.degree() == 0,
                "Node {} can't contains childs!",
                &node.data()
            );
        }

        Ok(result)
    }
}

fn parse_term(data: &str, node_type: &NodeType) -> Term {
    if let Ok(value) = Decimal::from_str(data) {
        Term::Number(value)
    } else if let Some(symbol) = symbol_by_name(data) {
        Term::Symbol(symbol.id)
    } else {
        match node_type {
            NodeType::Rule => Term::Param(Param::from_str(data).expect("unable to create param")),
            NodeType::Statement => {
                Term::Variable(Variable::from_str(data).expect("unable to create variable"))
            }
        }
    }
}

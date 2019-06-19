use std::collections::HashMap;
use std::fmt;

use parser::syntax_tree::Node as ParserNode;

use super::node::{Node, NodeType};
use super::symbols::all_symbols;

pub struct Statement {
    pub root: Node,
}

pub type ParamsMap = HashMap<String, u64>;

impl Statement {
    pub fn new(statement: &ParserNode, params: &mut ParamsMap) -> Option<Statement> {
        let mut params_count: u64 = *params.values().max().unwrap_or(&0);
        match Statement::parse_node(&statement, params, &mut params_count) {
            Some(root) => Some(Statement { root }),
            None => None,
        }
    }

    fn parse_node(
        src_node: &ParserNode,
        params: &mut ParamsMap,
        last_param_id: &mut u64,
    ) -> Option<Node> {
        let mut result = Node::new();
        let id = all_symbols().id_by_name(&src_node.label);
        match id {
            Some(i) => {
                result.node_type = NodeType::Symbol;
                result.id = i;
            }
            None => {
                result.node_type = NodeType::Param;
                if params.contains_key(&src_node.label) {
                    result.id = *params.get(&src_node.label).unwrap();
                } else {
                    *last_param_id += 1;
                    params.insert(src_node.label.clone(), *last_param_id);
                    result.id = *last_param_id;
                }
                if !src_node.childs.is_empty() {
                    error!(
                        "Node type Param({}) can't contains childs!",
                        &src_node.label
                    );
                    return None;
                }
            }
        }
        for child in &src_node.childs {
            match Statement::parse_node(&child, params, last_param_id) {
                Some(n) => result.childs.push(Box::new(n)),
                None => error!("Child parsing error: {}", &child.label),
            }
        }
        Some(result)
    }

    pub fn to_string(node: &Node) -> String {
        let mut result: String;
        match &node.node_type {
            NodeType::Symbol => {
                result = all_symbols()
                    .name_by_id(node.id)
                    .unwrap_or(String::from("unknown"))
                    .clone()
            }
            NodeType::Param => result = format!("p{}", node.id),
            NodeType::Varible => result = format!("v{}", node.id),
        }
        if node.childs.len() > 0 {
            result = format!(
                "{}({})",
                result,
                node.childs
                    .iter()
                    .map(|x| Statement::to_string(x))
                    .collect::<Vec<String>>()
                    .join(", ")
            );
        }
        result
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Statement::to_string(&self.root))
    }
}

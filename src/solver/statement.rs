use std::{collections::HashMap, fmt};

use super::trees::{Node, Tree};

use core::tree_utils::{parse_node, NodeData};

pub type ParamsMap = HashMap<String, u64>;
type ParserNode = Node<String>;
type StatementTree = Tree<NodeData>;

#[derive(Clone, Debug)]
pub struct Statement {
    pub root: StatementTree,
}

impl Statement {
    pub fn new(statement: &ParserNode, params: &mut ParamsMap) -> Result<Statement, String> {
        let mut params_count: u64 = *params.values().max().unwrap_or(&0);

        Ok(Statement {
            root: parse_node(&statement, params, &mut params_count)?,
        })
    }

    // pub fn to_string(node: &Node) -> String {
    //    let mut result: String;
    //    match &node.node_type {
    //        NodeType::Symbol => {
    //            result = all_symbols()
    //                .name_by_id(node.id)
    //                .unwrap_or(String::from("unknown"))
    //                .clone()
    //        }
    //        NodeType::Param => result = format!("p{}", node.id),
    //        NodeType::Varible => result = format!("v{}", node.id),
    //    }
    //    if node.childs.len() > 0 {
    //        result = format!(
    //            "{}({})",
    //            result,
    //            node.childs
    //                .iter()
    //                .map(|x| Statement::to_string(x))
    //                .collect::<Vec<String>>()
    //                .join(", ")
    //        );
    //    }
    //    result
    //}
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", &self.root)
    }
}

use std::collections::HashMap;

use parser::syntax_tree::Node as ParserNode;

use super::node::{Node, NodeType};
use super::symbols::all_symbols;

extern crate log;

pub struct Rule {
    pattern: Node,
    replace: Node,
}

type ParamsMap = HashMap<String, u64>;

impl Rule {
    pub fn new(statement: &ParserNode) -> Option<Rule> {
        if statement.label != "=>" {
            return None;
        }
        if statement.childs.len() != 2 {
            error!("Incorrect childs count: {}, should be 2!", statement.label);
            return None;
        }
        let mut params = HashMap::new();
        let mut params_count: u64 = 0;
        let left = Rule::parse_node(&statement.childs[0], &mut params, &mut params_count);
        if left.is_none() {
            return None;
        }
        let right = Rule::parse_node(&statement.childs[1], &mut params, &mut params_count);
        if right.is_none() {
            return None;
        }
        Some(Rule {
            pattern: left.unwrap(),
            replace: right.unwrap(),
        })
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
            match Rule::parse_node(&child, params, last_param_id) {
                Some(n) => result.childs.push(Box::new(n)),
                None => error!("Child parsing error: {}", &child.label),
            }
        }
        Some(result)
    }
}

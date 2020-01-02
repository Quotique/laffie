use std::collections::HashMap;

use super::{
    tree_utils::{apply_map, params_map, NodeData, parse_node},
    trees::{Node, Tree},
};

type ParamsMap = HashMap<String, u64>;
type ParserTree = Tree<String>;
type RuleTree = Tree<NodeData>;
type RuleNode = Node<NodeData>;

pub struct Rule {
    pub pattern: RuleTree,
    pub replace: RuleTree,
}

impl Rule {
    pub fn new(statement: &ParserTree) -> Option<Rule> {
        if statement.root().data != "=>" {
            return None;
        }
        if statement.degree() != 2 {
            error!("Incorrect childs count: {}, should be 2!", statement.root().data);
            return None;
        }
        let mut params = HashMap::new();
        let mut params_count: u64 = 0;
        let left = parse_node(statement.first().unwrap(), &mut params, &mut params_count);
        if left.is_none() {
            return None;
        }
        let right = parse_node(statement.last().unwrap(), &mut params, &mut params_count);
        if right.is_none() {
            return None;
        }
        Some(Rule {
            pattern: left.unwrap(),
            replace: right.unwrap(),
        })
    }

    pub fn apply(&self, arg: &RuleNode) -> Result<RuleTree, String> {
        let map = params_map(arg, &self.pattern)?;

        let mut result = self.replace.clone();
        apply_map(&mut result, &map);

        Ok(result)
    }
}

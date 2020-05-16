use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    convert::From,
    fmt,
    sync::{Arc, RwLock},
};

use super::trees::{Node, Tree};

use core::{
    rule::Rule,
    tree_utils::{parse_node, NodeData, swap_node},
};
// use std::borrow::BorrowMut;

pub type ParamsMap = HashMap<String, u64>;
type ParserNode = Node<String>;
type StatementTree = Tree<NodeData>;

pub const DEFAULT_WEIGHT: usize = 10;

#[derive(Clone, Debug)]
pub struct Statement {
    weight:        RefCell<usize>,
    applied_rules: RefCell<HashSet<usize>>,

    pub parents: Vec<Arc<Statement>>,
    pub rule:    Option<Arc<RwLock<Rule>>>,

    pub symbols: HashSet<u64>,
    pub root:    StatementTree,
}

impl Statement {
    pub fn new(statement: &ParserNode, params: &mut ParamsMap) -> Result<Statement, String> {
        let mut params_count: u64 = *params.values().max().unwrap_or(&0);

        let root = parse_node(&statement, params, &mut params_count)?;
        Ok(Statement {
            weight:        RefCell::new(DEFAULT_WEIGHT),
            applied_rules: RefCell::new(HashSet::new()),
            parents:       vec![],
            rule:          None,
            symbols:       Self::symbols(&root),
            root:          root,
        })
    }

    pub fn apply(statement: Arc<Self>, rule: Arc<RwLock<Rule>>) -> Result<Statement, String> {
        if statement
            .applied_rules
            .borrow_mut()
            .insert(rule.read().expect("Cant lock rule").id)
        {
            let new_tree = rule.read().expect("Cant lock rule").apply(&statement.root)?;
            Ok(Statement {
                weight:        RefCell::new(DEFAULT_WEIGHT),
                applied_rules: RefCell::new(HashSet::new()),
                parents:       vec![statement.clone()],
                rule:          Some(rule.clone()),
                symbols:       Self::symbols(&new_tree),
                root:          new_tree,
            })
        } else {
            let mut applied = false;
            let mut new_tree = statement.root.clone();
            for i in new_tree.iter_mut() {
                if let Ok(mut new_sub) = rule.read().expect("Cant lock rule").apply(&i) {
                    applied = true;
                    swap_node(i, &mut new_sub);
                }
            }
            if applied {
                Ok(Statement {
                    weight:        RefCell::new(DEFAULT_WEIGHT),
                    applied_rules: RefCell::new(HashSet::new()),
                    parents:       vec![statement.clone()],
                    rule:          Some(rule.clone()),
                    symbols:       Self::symbols(&new_tree),
                    root:          new_tree,
                })
            } else {
                Err("Rule applied".into())
            }
        }
    }

    pub fn weigth(&self) -> usize {
        *self.weight.borrow()
    }

    pub fn decrease_weigth(&self) -> bool {
        if *self.weight.borrow() == 0 {
            return false;
        }

        *self.weight.borrow_mut() -= 1;
        true
    }

    fn symbols(root: &StatementTree) -> HashSet<u64> {
        let mut symbols = HashSet::new();
        root.iter().for_each(|x| {
            if let NodeData::Symbol(s) = x.data {
                symbols.insert(s);
            }
        });
        symbols
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

impl From<StatementTree> for Statement {
    fn from(root: StatementTree) -> Self {
        Statement {
            weight: RefCell::new(DEFAULT_WEIGHT),
            applied_rules: RefCell::new(HashSet::new()),
            parents: vec![],
            rule: None,
            symbols: Self::symbols(&root),
            root,
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.root)
    }
}

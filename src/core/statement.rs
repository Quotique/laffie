use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    convert::From,
    fmt,
    sync::{Arc, RwLock},
};

use trees::Node;

use super::{
    rule::{Rule, RuleFlags},
    term::{display_string, parse_statement_node, StatementTree, Term},
    tree_utils::swap_node,
};

pub type ParamsMap = HashMap<String, u64>;

#[derive(Clone, Debug)]
pub struct Statement {
    applied_rules: RefCell<HashSet<usize>>,
    as_rule:       RefCell<bool>,

    pub parents: Vec<Arc<Statement>>,
    pub rule:    Option<Arc<RwLock<Rule>>>,

    pub symbols: HashSet<u64>,
    pub root:    StatementTree,
}

impl Statement {
    pub fn new(statement: &Node<String>, params: &mut ParamsMap) -> Result<Statement, String> {
        let mut params_count: u64 = *params.values().max().unwrap_or(&0);

        let root = parse_statement_node(&statement, params, &mut params_count)?;
        Ok(Statement {
            applied_rules: RefCell::new(HashSet::new()),
            as_rule:       RefCell::new(true),
            parents:       vec![],
            rule:          None,
            symbols:       Self::symbols(&root),
            root:          root,
        })
    }

    pub fn apply(statement: Arc<Self>, rule: Arc<RwLock<Rule>>) -> Result<Statement, String> {
        if !statement
            .applied_rules
            .borrow_mut()
            .insert(rule.read().expect("Cant lock rule").id)
        {
            return Err("Already applied".into());
        }
        if let Ok(new_tree) = rule.read().expect("Cant lock rule").apply(&statement.root) {
            Ok(Statement {
                applied_rules: RefCell::new(HashSet::new()),
                as_rule:       RefCell::new(true),
                parents:       vec![statement.clone()],
                rule:          Some(rule.clone()),
                symbols:       Self::symbols(&new_tree),
                root:          new_tree,
            })
        } else {
            // if subtree replacement
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
                    applied_rules: RefCell::new(HashSet::new()),
                    as_rule:       RefCell::new(true),
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

    pub fn rule(&self) -> Option<Rule> {
        if *self.as_rule.borrow() {
            *self.as_rule.borrow_mut() = false;

            if self.root.data.is_symbol_name(&"==".into()) {
                if self.root.first().unwrap().data.is_variable() {
                    return Some(Rule {
                        id:           0,
                        level:        0,
                        flags:        RuleFlags::SUBTREE_REPLACEMENT,
                        pattern:      self.root.first().unwrap().to_owned(),
                        replace:      self.root.last().unwrap().to_owned(),
                        requirements: vec![],
                    });
                }
            } else if self.root.data.is_symbol_name(&"=>".into()) {
                return Some(Rule {
                    id:           0,
                    level:        0,
                    flags:        RuleFlags::NONE,
                    pattern:      self.root.first().unwrap().to_owned(),
                    replace:      self.root.last().unwrap().to_owned(),
                    requirements: vec![],
                });
            }

            return None;
        }
        None
    }

    pub fn block_rule(&self, id: usize) {
        self.applied_rules.borrow_mut().insert(id);
    }

    fn symbols(root: &StatementTree) -> HashSet<u64> {
        let mut symbols = HashSet::new();
        for i in root.root().bfs().iter {
            if let Term::Symbol(s) = i.data {
                symbols.insert(*s);
            }
        }
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
            applied_rules: RefCell::new(HashSet::new()),
            as_rule: RefCell::new(true),
            parents: vec![],
            rule: None,
            symbols: Self::symbols(&root),
            root,
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", display_string(&self.root))
    }
}

#[cfg(test)]
mod statement_test {
    use super::*;
    use core::symbols::{symbol_by_name, symbols_tests::setup};
    use trees::tr;

    #[test]
    fn symbols_test() {
        setup();
        let test = tr(String::from("==")) /
            (tr(String::from("+")) /
                (tr(String::from("*")) / tr(String::from("2")) / tr(String::from("x"))) /
                tr(String::from("5"))) /
            tr(String::from("0"));
        let mut t1 = HashMap::new();
        let mut t2: u64 = 0;
        let state = parse_statement_node(&test, &mut t1, &mut t2).unwrap();
        let expect_syms = vec![String::from("=="), String::from("*"), String::from("+")];
        let syms = Statement::symbols(&state);
        assert_eq!(syms.len(), expect_syms.len());
        for s in expect_syms {
            let id = symbol_by_name(&s).unwrap().id;
            assert!(syms.contains(&id));
        }
    }
}

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    convert::From,
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
};

use trees::Node;

use super::{
    rule::{Rule, RuleAttr, RuleAttrValue},
    term::{display_string, parse_rule_node, parse_statement_node, StatementTree, Term},
    tree_utils::symbols,
};

pub type ParamsMap = HashMap<String, u64>;

#[derive(Clone, Debug)]
pub struct Statement {
    as_rule: RefCell<bool>,

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
            as_rule: RefCell::new(true),
            parents: vec![],
            rule:    None,
            symbols: symbols(&root),
            root:    root,
        })
    }

    pub fn new_with_params(
        statement: &Node<String>,
        params: &mut ParamsMap,
    ) -> Result<Statement, String> {
        let mut params_count: u64 = *params.values().max().unwrap_or(&0);

        let root = parse_rule_node(&statement, params, &mut params_count)?;
        Ok(Statement {
            as_rule: RefCell::new(true),
            parents: vec![],
            rule:    None,
            symbols: symbols(&root),
            root:    root,
        })
    }

    pub fn with_rule(mut self, rule: Arc<RwLock<Rule>>) -> Self {
        self.rule = Some(rule);
        self
    }

    pub fn rule(&self) -> Option<Rule> {
        if *self.as_rule.borrow() {
            *self.as_rule.borrow_mut() = false;

            if self.root.data.is_symbol_name(&"==".into()) {
                if self.root.first().unwrap().data.is_variable() {
                    if !Self::contains(&self.root.first().unwrap().data, &self.root.last().unwrap())
                    {
                        let pattern = self.root.first().unwrap().to_owned();
                        let pattern_symbols = symbols(&pattern);
                        return Some(Rule {
                            id:              0,
                            level:           0,
                            attrs:           [(RuleAttr::Subtree, RuleAttrValue::None)]
                                .iter()
                                .cloned()
                                .collect(),
                            pattern:         pattern,
                            replace:         self.root.last().unwrap().to_owned(),
                            requirements:    vec![],
                            pattern_symbols: pattern_symbols,
                        });
                    }
                }
            } else if self.root.data.is_symbol_name(&"=>".into()) {
                let pattern = self.root.first().unwrap().to_owned();
                let pattern_symbols = symbols(&pattern);
                return Some(Rule {
                    id:              0,
                    level:           0,
                    attrs:           HashMap::new(),
                    pattern:         pattern,
                    replace:         self.root.last().unwrap().to_owned(),
                    requirements:    vec![],
                    pattern_symbols: pattern_symbols,
                });
            }

            return None;
        }
        None
    }

    fn contains(term: &Term, tree: &Node<Term>) -> bool {
        if &tree.data == term {
            return true;
        }

        for i in tree.iter() {
            if Self::contains(term, i) {
                return true;
            }
        }
        false
    }
}

impl Hash for Statement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.root.hash(state);
    }
}

impl From<StatementTree> for Statement {
    fn from(root: StatementTree) -> Self {
        Statement {
            as_rule: RefCell::new(true),
            parents: vec![],
            rule: None,
            symbols: symbols(&root),
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
        let syms = symbols(&state);
        assert_eq!(syms.len(), expect_syms.len());
        for s in expect_syms {
            let id = symbol_by_name(&s).unwrap().id;
            assert!(syms.contains(&id));
        }
    }
}

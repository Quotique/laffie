use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::Path,
    sync::{Arc, RwLock},
};

use parser::lang;

use super::{
    symbols::{add_symbol, symbol_by_name, Symbol},
    tree_utils::{apply_map, params_map, parse_node, NodeData},
    trees::{Node, Tree},
};

// type ParamsMap = HashMap<String, u64>;
type ParserTree = Tree<String>;
type RuleTree = Tree<NodeData>;
type RuleNode = Node<NodeData>;

#[derive(Debug)]
pub struct Rule {
    pub id:      usize,
    pub level:   usize,
    pub pattern: RuleTree,
    pub replace: RuleTree,
}

pub struct RulesEngine {
    pub rules_by_sym: HashMap<u64, Vec<Arc<RwLock<Rule>>>>,
    last_rule_id:     usize,
}

impl Rule {
    pub fn new(id: usize, statement: &ParserTree) -> Result<Rule, String> {
        if statement.root().data != "=>" {
            return Err(format!("Expect => in rule, found: {:?}", statement.root()));
        }
        if statement.degree() != 2 {
            return Err(format!(
                "Incorrect childs count: {}, should be 2!",
                statement.root().data
            ));
        }
        let mut params = HashMap::new();
        let mut params_count: u64 = 0;
        let left = parse_node(statement.first().unwrap(), &mut params, &mut params_count)?;
        let right = parse_node(statement.last().unwrap(), &mut params, &mut params_count)?;
        Ok(Rule {
            id:      id,
            level:   0, // TODO: level impl
            pattern: left,
            replace: right,
        })
    }

    pub fn apply(&self, arg: &RuleNode) -> Result<RuleTree, String> {
        let map = params_map(arg, &self.pattern)?;

        let mut result = self.replace.clone();
        apply_map(&mut result, &map);

        Ok(result)
    }
}

impl RulesEngine {
    pub fn new() -> RulesEngine {
        RulesEngine {
            rules_by_sym: HashMap::new(),
            last_rule_id: 0,
        }
    }

    pub fn find_rules(&self, symbols: &HashSet<u64>) -> Vec<Arc<RwLock<Rule>>> {
        let mut rules = vec![];
        for i in symbols {
            match self.rules_by_sym.get(i) {
                Some(x) => rules.append(&mut x.clone()),
                None => {}
            }
        }
        rules
    }

    pub fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        if !dir.is_dir() {
            panic!(dir.to_string_lossy().to_string().push_str(" is not directory!"));
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.load_dir(&path)?;
            } else if path.extension().unwrap() == "sym" {
                self.load_file(&path)?;
            }
        }
        Ok(())
    }

    fn load_file(&mut self, file: &Path) -> io::Result<()> {
        info!("Processing file: {}", file.to_string_lossy());
        let content = fs::read_to_string(file)?;
        let states = lang::StatementsParser::new().parse(&content[..]).unwrap();
        let mut symbol_id: u64 = 0;
        for state in states {
            let s = state.root();
            if s.data == "Declare" && s.degree() == 2 && s.last().unwrap().data == "Symbol" {
                symbol_id = symbol_by_name(&s.first().unwrap().data).map(|s| s.id).unwrap_or(0);
                if !self.rules_by_sym.contains_key(&symbol_id) {
                    self.rules_by_sym.insert(symbol_id, Vec::new());
                }
            } else {
                trace!("Processing: {:?} ({})", s, symbol_id);
                self.last_rule_id += 1;
                match Rule::new(self.last_rule_id, &state) {
                    Ok(r) => self
                        .rules_by_sym
                        .get_mut(&symbol_id)
                        .unwrap()
                        .push(Arc::new(RwLock::new(r))),
                    Err(e) => trace!("Not rule!: {}", e),
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod rule_tests {
    use super::*;
    use core::{symbols::symbols_tests::setup, tree_utils::NodeData, trees::linked::fully::tr};
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn test_rule_tree() -> Tree<String> {
        tr(String::from("=>")) /
            (tr(String::from("==")) /
                (tr(String::from("+")) / tr(String::from("a")) / tr(String::from("x"))) /
                tr(String::from("0"))) /
            (tr(String::from("==")) / tr(String::from("x")) / (tr(String::from("-")) / tr(String::from("a"))))
    }

    #[test]
    fn pattern_test() {
        setup();
        let rule = Rule::new(1, &test_rule_tree()).expect("Unable to parse rule");
        assert_eq!(
            rule.pattern,
            tr(NodeData::Symbol(1)) /
                (tr(NodeData::Symbol(2)) / tr(NodeData::Param(1)) / tr(NodeData::Param(2))) /
                tr(NodeData::Symbol(4))
        );
    }

    #[test]
    fn replace_test() {
        setup();
        let rule = Rule::new(1, &test_rule_tree()).expect("Unable to parse rule");
        assert_eq!(
            rule.replace,
            tr(NodeData::Symbol(1)) / tr(NodeData::Param(2)) / (tr(NodeData::Symbol(3)) / tr(NodeData::Param(1)))
        );
    }

    #[test]
    fn apply_test() {
        setup();
        let rule = Rule::new(1, &test_rule_tree()).expect("Unable to parse rule");
        let state = tr(NodeData::Symbol(1)) /
            (tr(NodeData::Symbol(2)) / tr(NodeData::Symbol(5)) / tr(NodeData::Varible(1))) /
            tr(NodeData::Symbol(4));
        match rule.apply(state.root()) {
            Ok(result) => assert_eq!(
                result,
                tr(NodeData::Symbol(1)) /
                    tr(NodeData::Varible(1)) /
                    (tr(NodeData::Symbol(3)) / tr(NodeData::Symbol(5)))
            ),
            Err(e) => assert!(false, ""),
        }
    }
}

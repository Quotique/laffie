use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    fmt, fs, io,
    path::Path,
    sync::{Arc, RwLock},
};

use trees::Node;

use parser::{lang, Tree as ParserTree};

use super::{
    statement::{ParamsMap, Statement},
    symbols::Symbol,
    term::{display_string, parse_rule_node, ParamsNameMap, StatementTree, Term},
    tree_utils::{apply_map, params_map, symbols},
};

use crate::solver::problem::{ProblemType, ProblemTypeBuilder};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RuleAttr {
    Subtree,
    Equivalence,
    Replace,
    Target,
}

#[derive(Clone, Debug)]
pub enum RuleAttrValue {
    None,
    UInt(u64),
    Str(String),
    Target(ProblemType),
}

pub struct RuleBuilder {
    id: usize,
}

#[derive(Debug)]
pub struct Rule {
    pub id:    usize,
    pub level: usize,

    pub attrs: HashMap<RuleAttr, RuleAttrValue>,

    pub pattern: StatementTree,
    pub replace: StatementTree,

    pub requirements: Vec<StatementTree>,

    pub pattern_symbols: HashSet<u64>,
}

pub struct RulesEngine {
    pub rules_by_sym: HashMap<u64, Vec<Arc<RwLock<Rule>>>>,
    last_rule_id:     usize,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} => {} id: {}, level: {}, reqs: {:?}",
            display_string(self.pattern.root()),
            display_string(self.replace.root()),
            self.id,
            self.level,
            self.requirements
        )
    }
}

impl RuleBuilder {
    pub fn new() -> Self {
        Self { id: 0 }
    }

    pub fn with_id(mut self, id: usize) -> Self {
        self.id = id;
        self
    }

    pub fn with_statement(self, statement: &ParserTree) -> Result<Rule, String> {
        let mut rule = None;
        match statement.root().data.as_str() {
            "=>" | "<=>" => {
                RuleBuilder::parse_rule(statement.root(), &mut ParamsNameMap::new(), &mut 0).map(
                    |mut x| {
                        x.id = self.id;
                        x
                    },
                )
            }
            "Rule" => {
                let mut params = HashMap::new();
                let mut params_count: u64 = 0;

                let mut reqs = vec![];
                let mut attrs = vec![];

                for i in statement.forest().iter() {
                    match i.data.as_str() {
                        "=>" | "<=>" => {
                            rule =
                                Some(RuleBuilder::parse_rule(i, &mut params, &mut params_count)?);
                        }
                        "Predicates" => {
                            for k in i.iter() {
                                reqs.push(parse_rule_node(k, &mut params, &mut params_count)?)
                            }
                        }
                        "Attributes" => {
                            for k in i.iter() {
                                attrs.push(Self::parse_attribute(k, &mut params)?);
                            }
                        }
                        _ => return Err(format!("Bad rule symbol: {}", i.data)),
                    }
                }

                let mut rule = rule.expect("No rule found in Rule");

                rule.id = self.id;
                rule.requirements = reqs;
                rule.attrs.extend(attrs.iter().cloned());
                Ok(rule)
            }
            _ => Err(format!("Expect => in rule, found: {:?}", statement.root())),
        }
    }

    fn parse_attribute(
        attr: &Node<String>,
        params: &mut ParamsMap,
    ) -> Result<(RuleAttr, RuleAttrValue), String> {
        match attr.data.as_str() {
            "subtree" => Ok((RuleAttr::Subtree, RuleAttrValue::None)),
            "equivalence" => Ok((RuleAttr::Equivalence, RuleAttrValue::None)),
            "replace" => Ok((RuleAttr::Replace, RuleAttrValue::None)),
            "problem_target" => {
                if attr.degree() != 1 {
                    return Err("Bad target tree".into());
                }
                let problem = ProblemTypeBuilder::new(attr).with_params(params).rule()?;
                Ok((RuleAttr::Target, RuleAttrValue::Target(problem)))
            }
            _ => Err(format!("Incorrect attribute: {}", attr.data.as_str())),
        }
    }

    fn parse_rule(
        statement: &Node<String>,
        params: &mut ParamsNameMap,
        params_count: &mut u64,
    ) -> Result<Rule, String> {
        if statement.degree() != 2 {
            return Err(format!(
                "Incorrect childs count: {}, should be 2!",
                statement.data
            ));
        }
        let left = parse_rule_node(statement.first().unwrap(), params, params_count)?;
        let right = parse_rule_node(statement.last().unwrap(), params, params_count)?;

        let pattern_symbols = symbols(&left);
        Ok(Rule {
            id: 0,
            level: 0, // TODO: level
            attrs: if statement.data == "=>" {
                HashMap::new()
            } else {
                [(RuleAttr::Equivalence, RuleAttrValue::None)]
                    .iter()
                    .cloned()
                    .collect()
            },
            pattern: left,
            replace: right,
            requirements: vec![],
            pattern_symbols,
        })
    }
}

impl Rule {
    pub fn apply(&self, arg: &Node<Term>) -> Result<Vec<(StatementTree, Vec<Statement>)>, String> {
        let maps = params_map(arg, &self.pattern)?;

        Ok(maps
            .iter()
            .map(|x| {
                let mut result = self.replace.clone();
                apply_map(&mut result, &x);
                (
                    result,
                    self.requirements
                        .iter()
                        .map(|r| {
                            let mut r = r.clone();
                            apply_map(&mut r, &x);
                            Statement::from(r)
                        })
                        .collect(),
                )
            })
            .collect())
    }

    pub fn attribute(&self, attr: &RuleAttr) -> Option<&RuleAttrValue> {
        self.attrs.get(attr)
    }

    fn parse_rule(statement: &ParserTree) -> Result<(StatementTree, StatementTree), String> {
        if statement.degree() != 2 {
            return Err(format!(
                "Incorrect childs count: {}, should be 2!",
                statement.root().data
            ));
        }
        let mut params = HashMap::new();
        let mut params_count: u64 = 0;
        Ok((
            parse_rule_node(statement.first().unwrap(), &mut params, &mut params_count)?,
            parse_rule_node(statement.last().unwrap(), &mut params, &mut params_count)?,
        ))
    }
}

impl RulesEngine {
    pub fn new() -> RulesEngine {
        RulesEngine {
            rules_by_sym: HashMap::new(),
            last_rule_id: 0,
        }
    }

    pub fn find_rules(
        &self,
        symbols: &HashSet<u64>,
        applied_rules: &HashSet<usize>,
        blocked_rules: &HashSet<usize>,
        target: &ProblemType,
    ) -> Vec<Arc<RwLock<Rule>>> {
        trace!(
            "Symbols: {} applied: {:?}",
            symbols
                .iter()
                .map(|x| crate::core::symbols::symbol_by_id(*x).unwrap().name)
                .collect::<Vec<String>>()
                .join(","),
            applied_rules
        );
        let mut rules = vec![];
        for i in symbols {
            self.rules_by_sym.get(i).map(|x| {
                trace!(
                    "For symbol: {}  rules: {:?}",
                    crate::core::symbols::symbol_by_id(*i).unwrap().name,
                    x.iter()
                        .map(|a| a.read().unwrap().id)
                        .collect::<Vec<usize>>()
                );
                rules.extend(
                    x.iter()
                        .filter(|r| {
                            Self::filter_rule(
                                &r.read().expect("Cant lock rule"),
                                symbols,
                                applied_rules,
                                blocked_rules,
                                target,
                            )
                        })
                        .map(|r| r.clone()),
                )
            });
        }
        trace!(
            "Rules: {}",
            rules
                .iter()
                .map(|x| format!("{}", x.read().unwrap().id))
                .collect::<Vec<String>>()
                .join(",")
        );
        rules
    }

    fn filter_rule(
        rule: &Rule,
        symbols: &HashSet<u64>,
        applied_rules: &HashSet<usize>,
        blocked_rules: &HashSet<usize>,
        target: &ProblemType,
    ) -> bool {
        if applied_rules.contains(&rule.id) {
            return false;
        }
        if blocked_rules.contains(&rule.id) {
            return false;
        }
        for s in rule.pattern_symbols.iter() {
            if !symbols.contains(s) {
                trace!("symbol: {} not in list: {:?} reject", s, symbols);
                return false;
            }
        }
        match rule.attribute(&RuleAttr::Target) {
            Some(RuleAttrValue::Target(x)) => {
                trace!("targets {} {}", x, target);
                if target.map(x).is_err() {
                    trace!("no match");
                    return false;
                }
            }
            _ => match target {
                ProblemType::Transform => {
					// Only transform rules for transform
                    return false;
                }
                _ => {}
            },
        }
        true
    }

    pub fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        if !dir.is_dir() {
            panic!(dir
                .to_string_lossy()
                .to_string()
                .push_str(" is not directory!"));
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
            if let Ok(sym) = Symbol::try_from(&state) {
                symbol_id = sym.id;
                if !self.rules_by_sym.contains_key(&sym.id) {
                    self.rules_by_sym.insert(sym.id, Vec::new());
                }
            } else {
                trace!("Processing: {:?} ({})", s, symbol_id);
                self.last_rule_id += 1;
                match RuleBuilder::new()
                    .with_id(self.last_rule_id.clone())
                    .with_statement(&state)
                {
                    Ok(r) => {
                        trace!("New rule: {:?}", r);
                        self.rules_by_sym
                            .get_mut(&symbol_id)
                            .unwrap()
                            .push(Arc::new(RwLock::new(r)))
                    }
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
    use bigdecimal::BigDecimal as Decimal;
    use std::str::FromStr;
    use trees::Tree;

    use core::{symbols::symbols_tests::setup, term::Term, trees::linked::fully::tr};
    use logger::{log_init, Config as LogConfig};

    fn test_rule_tree() -> Tree<String> {
        tr(String::from("=>")) /
            (tr(String::from("==")) /
                (tr(String::from("+")) / tr(String::from("a")) / tr(String::from("x"))) /
                tr(String::from("0"))) /
            (tr(String::from("==")) /
                tr(String::from("x")) /
                (tr(String::from("-")) / tr(String::from("a"))))
    }

    fn test_extended_tree() -> Tree<String> {
        tr(String::from("Rule")) /
            (tr(String::from("=>")) /
                (tr(String::from("==")) /
                    (tr(String::from("+")) / tr(String::from("a")) / tr(String::from("x"))) /
                    tr(String::from("0"))) /
                (tr(String::from("==")) /
                    tr(String::from("x")) /
                    (tr(String::from("-")) / tr(String::from("a"))))) /
            (tr(String::from("Predicates")) /
                (tr(String::from("!=")) / tr(String::from("a")) / tr(String::from("0"))))
    }

    fn test_extended_tree_with_attr() -> Tree<String> {
        tr(String::from("Rule")) /
            (tr(String::from("=>")) /
                (tr(String::from("==")) /
                    (tr(String::from("+")) / tr(String::from("a")) / tr(String::from("x"))) /
                    tr(String::from("0"))) /
                (tr(String::from("==")) /
                    tr(String::from("x")) /
                    (tr(String::from("-")) / tr(String::from("a"))))) /
            (tr(String::from("Attributes")) / tr(String::from("replace"))) /
            (tr(String::from("Predicates")) /
                (tr(String::from("!=")) / tr(String::from("a")) / tr(String::from("0"))))
    }

    #[test]
    fn pattern_test() {
        setup();
        let rule = RuleBuilder::new()
            .with_id(1)
            .with_statement(&test_rule_tree())
            .expect("Unable to parse rule");
        assert_eq!(
            rule.pattern,
            tr(Term::Symbol(1)) /
                (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Param(2))) /
                tr(Term::Number(Decimal::from_str("0").unwrap()))
        );

        let rule_ext = RuleBuilder::new()
            .with_id(1)
            .with_statement(&test_extended_tree())
            .expect("Unable to parse rule");
        assert_eq!(
            rule_ext.pattern,
            tr(Term::Symbol(1)) /
                (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Param(2))) /
                tr(Term::Number(Decimal::from_str("0").unwrap()))
        );

        let rule_ext_with_attr = RuleBuilder::new()
            .with_id(1)
            .with_statement(&test_extended_tree_with_attr())
            .expect("Unable to parse rule");
        assert_eq!(
            rule_ext_with_attr.pattern,
            tr(Term::Symbol(1)) /
                (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Param(2))) /
                tr(Term::Number(Decimal::from_str("0").unwrap()))
        );
    }

    #[test]
    fn replace_test() {
        setup();
        let rule = RuleBuilder::new()
            .with_id(1)
            .with_statement(&test_rule_tree())
            .expect("Unable to parse rule");
        assert_eq!(
            rule.replace,
            tr(Term::Symbol(1)) / tr(Term::Param(2)) / (tr(Term::Symbol(3)) / tr(Term::Param(1)))
        );

        let rule_ext = RuleBuilder::new()
            .with_id(1)
            .with_statement(&test_extended_tree())
            .expect("Unable to parse rule");
        assert_eq!(
            rule_ext.replace,
            tr(Term::Symbol(1)) / tr(Term::Param(2)) / (tr(Term::Symbol(3)) / tr(Term::Param(1)))
        );

        let rule_ext_with_attr = RuleBuilder::new()
            .with_id(1)
            .with_statement(&test_extended_tree_with_attr())
            .expect("Unable to parse rule");
        assert_eq!(
            rule_ext_with_attr.replace,
            tr(Term::Symbol(1)) / tr(Term::Param(2)) / (tr(Term::Symbol(3)) / tr(Term::Param(1)))
        );
    }

    #[test]
    fn attribute_test() {
        setup();
        let rule = RuleBuilder::new()
            .with_id(1)
            .with_statement(&test_rule_tree())
            .expect("Unable to parse rule");
        assert!(rule.attribute(&RuleAttr::Replace).is_none());

        let rule_ext = RuleBuilder::new()
            .with_id(1)
            .with_statement(&test_extended_tree())
            .expect("Unable to parse rule");
        assert!(rule_ext.attribute(&RuleAttr::Replace).is_none());

        let rule_ext_with_attr = RuleBuilder::new()
            .with_id(1)
            .with_statement(&test_extended_tree_with_attr())
            .expect("Unable to parse rule");
        assert!(rule_ext_with_attr.attribute(&RuleAttr::Replace).is_some());
    }

    #[test]
    fn apply_test() {
        setup();
        let rule = RuleBuilder::new()
            .with_id(1)
            .with_statement(&test_rule_tree())
            .expect("Unable to parse rule");
        let state = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Symbol(5)) / tr(Term::Variable(1))) /
            tr(Term::Number(Decimal::from_str("0").unwrap()));
        match rule.apply(state.root()) {
            Ok(result) => {
                let result = result.into_iter().map(|x| x.0).collect::<Vec<Tree<Term>>>();
                assert_eq!(result.len(), 2);
                assert!(result.contains(
                    &(tr(Term::Symbol(1)) /
                        tr(Term::Variable(1)) /
                        (tr(Term::Symbol(3)) / tr(Term::Symbol(5))))
                ));
                assert!(result.contains(
                    &(tr(Term::Symbol(1)) /
                        tr(Term::Symbol(5)) /
                        (tr(Term::Symbol(3)) / tr(Term::Variable(1))))
                ));
            }
            Err(e) => assert!(false, "Rule must be applied. Error: {}", e),
        }
    }
}

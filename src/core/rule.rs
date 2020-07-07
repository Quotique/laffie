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
    statement::Statement,
    symbols::Symbol,
    term::{display_string, parse_rule_node, StatementTree, Term},
    tree_utils::{apply_map, params_map},
};

bitflags! {
    pub struct RuleFlags: u32 {
        const SUBTREE_REPLACEMENT = 0b010;
        const EQUIVALENCE         = 0b001;
        const NONE                = 0b000;
    }
}

#[derive(Debug)]
pub struct Rule {
    pub id:    usize,
    pub level: usize,
    pub flags: RuleFlags,

    pub pattern: StatementTree,
    pub replace: StatementTree,

    pub requirements: Vec<StatementTree>,
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

pub struct RulesEngine {
    pub rules_by_sym: HashMap<u64, Vec<Arc<RwLock<Rule>>>>,
    last_rule_id:     usize,
}

impl Rule {
    pub fn new(rule_id: usize, statement: &ParserTree) -> Result<Rule, String> {
        let result = match statement.root().data.as_str() {
            "=>" | "<=>" => {
                let (left, right) = Rule::parse_rule(statement)?;
                Rule {
                    id:           rule_id,
                    level:        0, // TODO: level
                    flags:        if statement.root().data == "=>" {
                        RuleFlags::NONE
                    } else {
                        RuleFlags::EQUIVALENCE
                    },
                    pattern:      left,
                    replace:      right,
                    requirements: vec![],
                }
            }
            "Rule" => {
                let mut params = HashMap::new();
                let mut params_count: u64 = 0;

                let mut left = None;
                let mut right = None;
                let mut rule_flags = None;
                let mut reqs = vec![];

                for i in statement.iter() {
                    match i.data.as_str() {
                        "=>" | "<=>" => {
                            left = Some(parse_rule_node(
                                i.first().unwrap(),
                                &mut params,
                                &mut params_count,
                            )?);
                            right = Some(parse_rule_node(
                                i.last().unwrap(),
                                &mut params,
                                &mut params_count,
                            )?);

                            rule_flags = Some(if i.data.as_str() == "<=>" {
                                RuleFlags::EQUIVALENCE
                            } else {
                                RuleFlags::NONE
                            });
                        }
                        _ => reqs.push(parse_rule_node(i, &mut params, &mut params_count)?),
                    }
                }

                Rule {
                    id:           rule_id,
                    level:        0, // TODO: level
                    flags:        rule_flags.unwrap(),
                    pattern:      left.unwrap(),
                    replace:      right.unwrap(),
                    requirements: reqs,
                }
            }
            _ => {
                return Err(format!("Expect => in rule, found: {:?}", statement.root()));
            }
        };

        Ok(result)
    }

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
                match Rule::new(self.last_rule_id, &state) {
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
            (tr(String::from("!=")) / tr(String::from("a")) / tr(String::from("0")))
    }

    #[test]
    fn pattern_test() {
        setup();
        let rule = Rule::new(1, &test_rule_tree()).expect("Unable to parse rule");
        assert_eq!(
            rule.pattern,
            tr(Term::Symbol(1)) /
                (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Param(2))) /
                tr(Term::Number(Decimal::from_str("0").unwrap()))
        );

        let rule_ext = Rule::new(1, &test_extended_tree()).expect("Unable to parse rule");
        assert_eq!(
            rule_ext.pattern,
            tr(Term::Symbol(1)) /
                (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Param(2))) /
                tr(Term::Number(Decimal::from_str("0").unwrap()))
        );
    }

    #[test]
    fn replace_test() {
        setup();
        let rule = Rule::new(1, &test_rule_tree()).expect("Unable to parse rule");
        assert_eq!(
            rule.replace,
            tr(Term::Symbol(1)) / tr(Term::Param(2)) / (tr(Term::Symbol(3)) / tr(Term::Param(1)))
        );

        let rule_ext = Rule::new(1, &test_extended_tree()).expect("Unable to parse rule");
        assert_eq!(
            rule_ext.replace,
            tr(Term::Symbol(1)) / tr(Term::Param(2)) / (tr(Term::Symbol(3)) / tr(Term::Param(1)))
        );
    }

    #[test]
    fn apply_test() {
        setup();
        log_init(&LogConfig {
            filename: String::from("test.log"),
            level:    String::from("Trace"),
        });
        let rule = Rule::new(1, &test_rule_tree()).expect("Unable to parse rule");
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

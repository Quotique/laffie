use crate::{
    statement::{symbols::symbol_by_id, MarkedStatement, Statement},
    utils::VecDisplay,
};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    str::FromStr,
    sync::Arc,
};

use anyhow::{bail, Result};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RuleAttr {
    Subtree,
    Equivalence,
    Replace,
    Target,
    Level,
    Zero,
    One,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleAttrValue {
    None,
    UInt(u64),
    Target(Statement),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleDeclineReason {
    LevelMissmatch,
    TargetMissmatch,
    AlreadyApplied,
    Blocked,
    ParamsMappingErr(String),
}

#[derive(Debug)]
pub struct Suppose {
    pub requirements: Vec<Arc<Statement>>,
    pub resolution:   MarkedStatement,
}

#[derive(Debug)]
pub struct Rule {
    pub id:        usize,
    pub level:     usize,
    pub symbol_id: u64,

    pub attrs: HashMap<RuleAttr, RuleAttrValue>,

    pub pattern: Statement,
    pub replace: Statement,

    pub requirements: Vec<Statement>,

    pub pattern_symbols: HashSet<u64>,
}

impl FromStr for RuleAttr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "subtree" => Ok(RuleAttr::Subtree),
            "equivalence" => Ok(RuleAttr::Equivalence),
            "replace" => Ok(RuleAttr::Replace),
            "level" => Ok(RuleAttr::Level),
            "problem_target" => Ok(RuleAttr::Target),
            "zero" => Ok(RuleAttr::Zero),
            "one" => Ok(RuleAttr::One),
            _ => bail!(""),
        }
    }
}

impl fmt::Display for Suppose {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{}] => {}",
            VecDisplay(&self.requirements),
            self.resolution,
        )
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} => {} id: {}, level: {}, reqs: {:?}",
            self.pattern, self.replace, self.id, self.level, self.requirements
        )
    }
}

impl Rule {
    pub fn attribute(&self, attr: &RuleAttr) -> Option<&RuleAttrValue> {
        self.attrs.get(attr)
    }

    pub fn is_tautology(&self) -> bool {
        self.pattern == self.replace
    }

    pub fn is_statement_suitable(
        &self,
        statement: &MarkedStatement,
    ) -> Result<(), RuleDeclineReason> {
        if self.level != statement.weight {
            return Err(RuleDeclineReason::LevelMissmatch);
        } else if statement.applied_rules.contains(&self.id) {
            return Err(RuleDeclineReason::AlreadyApplied);
        } else if statement.blocked_rules.contains(&self.id) {
            return Err(RuleDeclineReason::Blocked);
        }

        for s in self.pattern_symbols.iter() {
            if !statement.symbols.contains(s) {
                return Err(RuleDeclineReason::ParamsMappingErr(format!(
                    "symbol: {} not found",
                    symbol_by_id(*s).unwrap().name
                )));
            }
        }

        Ok(())
    }

    pub fn is_target_suitable(&self, target: &MarkedStatement) -> Result<(), RuleDeclineReason> {
        if let Some(RuleAttrValue::Target(pattern)) = self.attribute(&RuleAttr::Target) {
            if pattern.map(&target.statement).is_err() {
                trace!(target: "rule_selection", "no match target: {}, required: {}", target, pattern);
                return Err(RuleDeclineReason::TargetMissmatch);
            }
            return Ok(());
        }
        if (*target.statement)
            .root()
            .data()
            .is_symbol_name("transform")
        {
            // Only transform rules for transform
            return Err(RuleDeclineReason::TargetMissmatch);
        }
        Ok(())
    }

    pub fn apply(
        &self,
        arg: &mut MarkedStatement,
        target: &MarkedStatement,
    ) -> Result<Vec<Suppose>, RuleDeclineReason> {
        let _ = self.is_statement_suitable(arg)?;
        let _ = self.is_target_suitable(target)?;

        if !arg.applied_rules.insert(self.id) {
            return Err(RuleDeclineReason::AlreadyApplied);
        }
        if arg.blocked_rules.contains(&self.id) {
            return Err(RuleDeclineReason::Blocked);
        }

        if self.attribute(&RuleAttr::Subtree).is_some() {
            self.apply_subtree(arg)
        } else {
            self.apply_root(arg)
        }
    }

    fn apply_root(&self, arg: &mut MarkedStatement) -> Result<Vec<Suppose>, RuleDeclineReason> {
        let maps = self
            .pattern
            .map(&arg.statement)
            .map_err(RuleDeclineReason::ParamsMappingErr)?;

        Ok(maps
            .iter()
            .map(|x| Suppose {
                requirements: self
                    .requirements
                    .iter()
                    .map(|r| Arc::new(r.apply_map(&x)))
                    .collect(),
                resolution:   MarkedStatement::from(Arc::new(
                    self.replace.apply_map(&x).normalize(),
                ))
                .with_parent(arg.id),
            })
            .collect())
    }

    fn apply_subtree(&self, arg: &mut MarkedStatement) -> Result<Vec<Suppose>, RuleDeclineReason> {
        let mut statement = (*arg.statement).clone();
        let state = &statement as *const Statement;
        let (maps, mut node) = self
            .pattern
            .find_subtree_map_mut(&mut statement)
            .ok_or_else(|| RuleDeclineReason::ParamsMappingErr("no match".into()))?;

        Ok(maps
            .iter()
            .map(|x| {
                let mut replace = self.replace.apply_map(&x);
                replace.swap_node(&mut node);
                let clone = unsafe { (*state).normalize() };
                let suppose = Suppose {
                    requirements: self
                        .requirements
                        .iter()
                        .map(|r| Arc::new(r.apply_map(&x)))
                        .collect(),
                    resolution:   MarkedStatement::from(Arc::new(clone)).with_parent(arg.id),
                };
                trace!(target: "rule_selection", "New suppose: {}", suppose);
                suppose
            })
            .collect())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::{
        parser::{ra, statement_with_vars, RuleParser, StatementParser},
        predefine::setup,
        statement::MarkedStatement,
    };
    use std::sync::Arc;

    fn base_rule() -> Rule {
        setup();
        let test_rule = r#"rule {
                            attr level(1);
                            a + x == 0 => x == -a;
                            a!=0;
                        }"#;

        let mut rules = RuleParser::with(&ra::lang_rule(test_rule).unwrap())
            .parse()
            .unwrap();
        assert_eq!(rules.len(), 1);
        rules.pop().unwrap()
    }

    fn subtree_rule() -> Rule {
        setup();
        let test_rule = r#"rule {
                            attr subtree,level(1);
                            --a <=> a;
                        }"#;

        let mut rules = RuleParser::with(&ra::lang_rule(test_rule).unwrap())
            .parse()
            .unwrap();
        assert_eq!(rules.len(), 1);
        rules.pop().unwrap()
    }

    fn test_statement() -> MarkedStatement {
        setup();
        let test_statement = r#"2 + x == 0"#;
        MarkedStatement::from(Arc::new(
            StatementParser::new(&ra::statements(test_statement).unwrap()[0])
                .with_variables()
                .parse()
                .unwrap(),
        ))
    }

    fn test_statement_subtree() -> MarkedStatement {
        setup();
        let test_statement = r#"x + (-(-2)) == 0"#;
        MarkedStatement::from(Arc::new(
            StatementParser::new(&ra::statements(test_statement).unwrap()[0])
                .with_variables()
                .parse()
                .unwrap(),
        ))
    }

    fn test_target() -> MarkedStatement {
        setup();
        let test_statement = r#"find(x)"#;
        MarkedStatement::from(Arc::new(
            StatementParser::new(&ra::statements(test_statement).unwrap()[0])
                .with_variables()
                .parse()
                .unwrap(),
        ))
    }

    #[test]
    fn level_comparsion_test() {
        let rule = base_rule();
        let mut statement = test_statement();
        let target = test_target();

        assert_eq!(
            rule.apply(&mut statement, &target).err(),
            Some(RuleDeclineReason::LevelMissmatch)
        );
    }

    #[test]
    fn apply_test() {
        let rule = base_rule();
        let mut statement = test_statement();
        let target = test_target();

        statement.weight = 1;
        let suppose = rule.apply(&mut statement, &target);
        assert!(suppose.is_ok());
        let suppose = suppose.unwrap();
        assert_eq!(suppose.len(), 2);
        assert_eq!(suppose[0].requirements.len(), 1);
        assert_eq!(*suppose[0].requirements[0], statement_with_vars("x != 0"));
        assert_eq!(
            *suppose[0].resolution.statement,
            statement_with_vars("2 == -x")
        );
        assert_eq!(suppose[1].requirements.len(), 1);
        assert_eq!(*suppose[1].requirements[0], statement_with_vars("2 != 0"));
        assert_eq!(
            *suppose[1].resolution.statement,
            statement_with_vars("x == -2").normalize()
        );
    }

    #[test]
    fn subtree_apply_test() {
        let rule = subtree_rule();
        let mut statement = test_statement_subtree();
        let target = test_target();

        statement.weight = 1;
        let suppose = rule.apply(&mut statement, &target);
        assert!(suppose.is_ok());
        let suppose = suppose.unwrap();
        assert_eq!(suppose.len(), 1);
        assert_eq!(suppose[0].requirements.len(), 0);
        assert_eq!(
            *suppose[0].resolution.statement,
            statement_with_vars("x + 2 == 0")
        );
    }

    #[test]
    fn twice_apply_test() {
        let rule = subtree_rule();
        let mut statement = test_statement_subtree();
        let target = test_target();

        statement.weight = 1;
        assert!(rule.apply(&mut statement, &target).is_ok());
        assert_eq!(
            rule.apply(&mut statement, &target).err(),
            Some(RuleDeclineReason::AlreadyApplied)
        );
    }
}

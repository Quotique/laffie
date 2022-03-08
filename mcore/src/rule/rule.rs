use std::{collections::HashSet, fmt, str::FromStr, sync::Arc};

use eyre::{bail, Result};
use multimap::MultiMap;

use crate::{
    predefine::symbol_by_id,
    statement::{
        CompactString, MarkedStatement, NodePosition, ParamsMapping, Statement, StatementNode,
    },
    utils::VecDisplay,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RuleAttr {
    Subtree,
    Equivalence,
    Replace,
    Target,
    Level,
    Zero,
    One,
    Block,
    Id,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleAttrValue {
    None,
    UInt(u64),
    Str(CompactString),
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
    pub id:        u64,
    pub level:     usize,
    pub symbol_id: u64,

    pub attrs: MultiMap<RuleAttr, RuleAttrValue>,
    pub block: Vec<u64>,

    pub statement: Statement,
    pub pattern:   NodePosition,
    pub replace:   NodePosition,
    pub binds:     ParamsMapping,

    pub requirements: Vec<Statement>,

    pub pattern_symbols: HashSet<u64>,
}

impl FromStr for RuleAttr {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "subtree" => Ok(RuleAttr::Subtree),
            "equivalence" => Ok(RuleAttr::Equivalence),
            "replace" => Ok(RuleAttr::Replace),
            "level" => Ok(RuleAttr::Level),
            "problem_target" => Ok(RuleAttr::Target),
            "zero" => Ok(RuleAttr::Zero),
            "one" => Ok(RuleAttr::One),
            "id" => Ok(RuleAttr::Id),
            "block" => Ok(RuleAttr::Block),
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
            "{} id: {}, level: {}, reqs: {}",
            self.statement,
            self.id,
            self.level,
            VecDisplay(&self.requirements)
        )
    }
}

impl Rule {
    pub fn attribute(&self, attr: &RuleAttr) -> impl Iterator<Item = &RuleAttrValue> {
        self.attrs.iter_key(attr)
    }

    pub fn contains_attribute(&self, attr: &RuleAttr) -> bool {
        self.attrs.iter_key(attr).next().is_some()
    }

    pub fn is_tautology(&self) -> bool {
        self.pattern_node() == self.replace_node()
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
        // TODO: multiple targets
        if let Some(RuleAttrValue::Target(pattern)) = self.attribute(&RuleAttr::Target).next() {
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

        if self.contains_attribute(&RuleAttr::Subtree) {
            self.apply_subtree(arg)
        } else {
            self.apply_root(arg)
        }
    }

    #[inline]
    pub fn pattern_node(&self) -> &StatementNode {
        &self.statement[&self.pattern]
    }

    #[inline]
    pub fn replace_node(&self) -> &StatementNode {
        &self.statement[&self.replace]
    }

    fn apply_root(&self, arg: &mut MarkedStatement) -> Result<Vec<Suppose>, RuleDeclineReason> {
        let maps = ParamsMapping::mapper(arg.statement.root(), self.pattern_node())
            .try_map()
            .map_err(|e| RuleDeclineReason::ParamsMappingErr(e.to_string()))?;

        Ok(maps
            .iter()
            .map(|x| Suppose {
                requirements: self
                    .requirements
                    .iter()
                    .map(|r| Arc::new(r.apply_map(x)))
                    .collect(),
                resolution:   MarkedStatement::from(Arc::new(
                    Statement::from(self.replace_node().deep_clone())
                        .apply_map(x)
                        .normalize(),
                ))
                .with_parent(arg.id),
            })
            .collect())
    }

    fn apply_subtree(&self, arg: &mut MarkedStatement) -> Result<Vec<Suppose>, RuleDeclineReason> {
        let maps = ParamsMapping::subtree_map(arg.statement.root(), self.pattern_node());
        if maps.is_empty() {
            return Err(RuleDeclineReason::ParamsMappingErr("no match".into()));
        }

        let mut result = vec![];
        for (maps, pos) in maps.iter() {
            for i in maps.iter() {
                let replace = Statement::from(self.replace_node().deep_clone());

                let mut replace = replace.apply_map(&self.binds).apply_map(i);
                let mut src = (*arg.statement).clone();
                replace.swap_node(&mut src[pos]);
                src.inpl_normalize();
                let mut resolution = MarkedStatement::from(Arc::new(src)).with_parent(arg.id);
                resolution.blocked_rules.extend(self.block.iter());

                let suppose = Suppose {
                    requirements: self
                        .requirements
                        .iter()
                        .map(|r| Arc::new(r.apply_map(&self.binds).apply_map(i)))
                        .collect(),
                    resolution,
                };
                trace!(target: "rule_selection", "New suppose: {}", suppose);
                result.push(suppose);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use crate::{
        rule::{parse_rule, Rule, RuleDeclineReason},
        statement::{statement_with_vars, MarkedStatement},
    };

    fn base_rule() -> Rule {
        parse_rule(
            r#"rule {
                attr level(1);
                a + x == 0 => x == -a;
                a!=0;
            }"#,
        )
    }

    fn subtree_rule() -> Rule {
        parse_rule(
            r#"rule {
                attr subtree,level(1);
                --a <=> a;
            }"#,
        )
    }

    fn rule_with_binds() -> Rule {
        parse_rule(
            r#"rule {
                attr level(1);
                a/((b + c) as D) == 0 <=> a == 0 && D != 0;
            }"#,
        )
    }

    fn test_statement_fraction() -> MarkedStatement {
        MarkedStatement::from(Arc::new(statement_with_vars(r#"2/(x + 1) == 0"#)))
    }

    fn test_statement() -> MarkedStatement {
        MarkedStatement::from(Arc::new(statement_with_vars(r#"2 + x == 0"#)))
    }

    fn test_statement_subtree() -> MarkedStatement {
        MarkedStatement::from(Arc::new(statement_with_vars(r#"x + (-(-2)) == 0"#)))
    }

    fn test_target() -> MarkedStatement {
        MarkedStatement::from(Arc::new(statement_with_vars(r#"find(x)"#)))
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
    fn subtree_apply_test_2() {
        let rule = parse_rule(
            r#"rule {
                attr level(0),problem_target(transform(x)),subtree,replace;
                a && b <=> b;

                a is true;
            }"#,
        );

        let test_statement = r#"(x^4 - 25*x^2 + 60*x -36 != 0) && ((3600 < 0 && x in empty_set) || (3600 >= 0 && x in set(1, 2)))"#;
        let mut statement = MarkedStatement::from(Arc::new(statement_with_vars(test_statement)));

        let target = MarkedStatement::from(Arc::new(statement_with_vars(r#"transform(a)"#)));
        statement.weight = 0;

        let suppose = rule.apply(&mut statement, &target);
        assert!(suppose.is_ok());
        let suppose = suppose.unwrap();
        assert_eq!(suppose.len(), 3);
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

    #[test]
    fn bind_apply_test() {
        let rule = rule_with_binds();
        let mut statement = test_statement_fraction();
        let target = test_target();

        statement.weight = 1;
        let suppose = rule.apply(&mut statement, &target).unwrap();
        assert_eq!(suppose[0].requirements.len(), 0);
        assert_eq!(
            *suppose[0].resolution.statement,
            statement_with_vars("2 == 0 && x + 1 != 0")
        );
    }
}

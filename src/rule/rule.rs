use crate::{
    solver::problem::ProblemType,
    statement::{MarkedStatement, Statement},
};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::{Arc, RwLock},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RuleAttr {
    Subtree,
    Equivalence,
    Replace,
    Target,
    Level,
}

#[derive(Clone, Debug)]
pub enum RuleAttrValue {
    None,
    UInt(u64),
    Str(String),
    Target(ProblemType),
}

#[derive(Clone, Debug)]
pub enum RuleDeclineReason {
    LevelMissmatch,
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
    pub fn apply(
        &self,
        arg: &Statement,
        frame: &Vec<MarkedStatement>,
    ) -> Result<Vec<MarkedStatement>, RuleDeclineReason> {
        Err(RuleDeclineReason::LevelMissmatch) // TODO
    }

    pub fn attribute(&self, attr: &RuleAttr) -> Option<&RuleAttrValue> {
        self.attrs.get(attr)
    }

    pub fn is_statement_suitable(&self, statement: &MarkedStatement) -> bool {
        if self.level != statement.weight ||
            statement.applied_rules.contains(&self.id) ||
            statement.blocked_rules.contains(&self.id)
        {
            return false;
        }

        for s in self.pattern_symbols.iter() {
            if !statement.symbols.contains(s) {
                return false;
            }
        }

        true
    }

    pub fn is_target_suitable(&self, target: &ProblemType) -> bool {
        match self.attribute(&RuleAttr::Target) {
            Some(RuleAttrValue::Target(x)) => {
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

    // pub fn apply(&self, arg: &Node<Term>) -> Result<Vec<(StatementTree,
    // Vec<Statement>)>, String> {     let maps = params_map(arg,
    // &self.pattern)?;
    //
    //     Ok(maps
    //         .iter()
    //         .map(|x| {
    //             let mut result = self.replace.clone();
    //             apply_map(&mut result, &x);
    //             (
    //                 result,
    //                 self.requirements
    //                     .iter()
    //                     .map(|r| {
    //                         let mut r = r.clone();
    //                         apply_map(&mut r, &x);
    //                         Statement::from(r)
    //                     })
    //                     .collect(),
    //             )
    //         })
    //         .collect())
    // }

    // fn parse_rule(statement: &ParserTree) -> Result<(StatementTree,
    // StatementTree), String> {     if statement.degree() != 2 {
    //         return Err(format!(
    //             "Incorrect childs count: {}, should be 2!",
    //             statement.root().data
    //         ));
    //     }
    //     let mut params = HashMap::new();
    //     let mut params_count: u64 = 0;
    //     Ok((
    //         parse_rule_node(statement.first().unwrap(), &mut params, &mut
    // params_count)?,         parse_rule_node(statement.last().unwrap(), &mut
    // params, &mut params_count)?,     ))
    // }
}

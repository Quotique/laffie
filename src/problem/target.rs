use super::{frame::Frame, solution::MAX_LEVEL};
use crate::{
    core::term::Term,
    parser::SemanticError,
    rule::{RuleAttr, RulesEngine, SharedRule, Suppose},
    solver::operations::is_true,
    statement::{MarkedStatement, Statement},
    utils::Dumper,
};
use std::{convert::TryFrom, sync::Arc};
use trees::tr;

pub enum Target {
    Find(MarkedStatement),
    Proof(Frame),
    Transform(Frame),
}

impl Target {
    pub fn try_from(mut value: Statement, rules: Arc<RulesEngine>) -> Result<Self, SemanticError> {
        let (root, mut childs) = value.destruct();

        if root.data.is_symbol_name(&"find".into()) {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(format!("")));
            }
            return Ok(Self::Find(MarkedStatement::from(Arc::new(
                Statement::from(childs.pop_front().unwrap()),
            ))));
        } else if root.data.is_symbol_name(&"proof".into()) {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(format!("")));
            }

            let mut frame = Frame::new(rules, Dumper::default());
            frame.add_condition(MarkedStatement::from(Arc::new(Statement::from(
                childs.pop_front().unwrap(),
            ))));
            return Ok(Self::Proof(frame));
        } else if root.data.is_symbol_name(&"transform".into()) {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(format!("")));
            }

            let mut frame = Frame::new(rules, Dumper::default());
            frame.add_condition(MarkedStatement::from(Arc::new(Statement::from(
                childs.pop_front().unwrap(),
            ))));
            return Ok(Self::Transform(frame));
        } else {
            Err(SemanticError::UnexpectedWord(root.to_string()))
        }
    }

    pub fn is_answer(&self, statement: &MarkedStatement) -> Option<Suppose> {
        let statement_root = statement.statement.root();

        match self {
            Self::Find(x) => {
                if statement_root.degree() != 2 ||
                    (!statement_root.data.is_symbol_name(&"==".into()) &&
                        !statement_root.data.is_symbol_name(&"in".into()))
                {
                    return None;
                }

                if statement_root.first().unwrap() == x.statement.root() {
                    let is_known = tr(Term::with_symbol_name("is").unwrap()) /
                        statement_root.last().unwrap().to_owned() /
                        tr(Term::with_symbol_name("known").unwrap());

                    return Some(Suppose {
                        requirements: vec![Arc::new(Statement::from(is_known))],
                        resolution:   statement.clone(),
                    });
                }
                None
            }
            Self::Proof(x) => {
                for i in x.iter() {
                    if statement_root == i.statement.root() {
                        return Some(Suppose {
                            requirements: vec![],
                            resolution:   statement.clone(),
                        });
                    }
                    if is_true(&i.statement.root()) {
                        return Some(Suppose {
                            requirements: vec![],
                            resolution:   statement.clone(),
                        });
                    }
                }
                None
            }
            Self::Transform(x) => {
                if let Ok(index) = x.pick_condition() {
                    if x[index].weight > MAX_LEVEL {
                        return Some(Suppose {
                            requirements: vec![],
                            resolution:   x.last().unwrap().clone(),
                        });
                    }
                }
                None
            }
        }
    }

    pub fn prepare_target(
        &mut self,
        level: usize,
        local_rules: Vec<SharedRule>,
        main_frame: &Frame,
        target: &MarkedStatement,
    ) {
        match self {
            Self::Find(_) => {}
            Self::Proof(x) => {
                while let Ok(index) = x.pick_condition() {
                    if x[index].weight > level {
                        return;
                    }
                    let new_states = main_frame.next_statement_with_statement(
                        local_rules.clone(),
                        &mut x[index],
                        target,
                        |rule| rule.attribute(&RuleAttr::Equivalence).is_some(),
                    );
                    if new_states.len() == 0 {
                        x[index].weight += 1;
                    }
                    for s in new_states {
                        let _ = x.add_condition(s);
                    }
                }
            }
            Self::Transform(x) => {
                while let Ok(index) = x.pick_condition() {
                    if x[index].weight > level {
                        return;
                    }
                    let new_states = main_frame.next_statement_with_statement(
                        local_rules.clone(),
                        &mut x[index],
                        target,
                        |_| true,
                    );
                    if new_states.len() == 0 {
                        x[index].weight += 1;
                    } else {
                        for s in new_states {
                            if x.add_condition(s).is_ok() {
                                x[index].weight = MAX_LEVEL + 1;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn is_transform(&self) -> bool {
        if let Target::Transform(_) = self {
            return true;
        }
        return false;
    }
}

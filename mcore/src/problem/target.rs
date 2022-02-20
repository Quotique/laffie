use std::{fmt, sync::Arc};

use trees::tr;

use crate::{
    rule::{RuleAttr, RulesEngine, SharedRule, Suppose},
    statement::{term::Term, tree_utils::NodeMapping, MarkedStatement, Statement},
    utils::Dumper,
};

use super::{frame::Frame, solution::MAX_LEVEL};

// TODO: remove
#[derive(Clone, Debug)]
pub enum SemanticError {
    UnexpectedWord(String),
    WorngArgCount(String),
}

pub enum Target {
    Find(MarkedStatement),
    Proof(Frame),
    Transform(Frame),
}

impl fmt::Debug for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Target::Find(s) => write!(f, "Find: {:?}", s),
            Target::Proof(s) => write!(f, "Proof: {:?}", s),
            Target::Transform(s) => write!(f, "Transform {:?}", s),
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Target::Find(s) => write!(f, "Find: {:?}", s),
            Target::Proof(s) => write!(f, "Proof: {:?}", s),
            Target::Transform(s) => write!(f, "Transform {:?}", s),
        }
    }
}

impl Target {
    pub fn try_from(value: Statement, rules: Arc<RulesEngine>) -> Result<Self, SemanticError> {
        let (root, mut childs) = value.destruct();

        if root.data().is_symbol_name("find") {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(String::default()));
            }
            Ok(Self::Find(MarkedStatement::from(Arc::new(
                Statement::from(childs.pop_front().unwrap()),
            ))))
        } else if root.data().is_symbol_name("proof") {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(String::default()));
            }

            let mut frame = Frame::new(rules, Dumper::default());
            // TODO: error Processing
            let _ = frame.add_condition(MarkedStatement::from(Arc::new(Statement::from(
                childs.pop_front().unwrap(),
            ))));
            Ok(Self::Proof(frame))
        } else if root.data().is_symbol_name("transform") {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(String::default()));
            }

            let mut frame = Frame::new(rules, Dumper::default());
            // TODO: error Processing
            let _ = frame.add_condition(MarkedStatement::from(Arc::new(Statement::from(
                childs.pop_front().unwrap(),
            ))));
            Ok(Self::Transform(frame))
        } else {
            Err(SemanticError::UnexpectedWord(root.to_string()))
        }
    }

    pub fn is_answer(&self, statement: &MarkedStatement) -> Option<Suppose> {
        let statement_root = statement.statement.root();

        if statement_root.data().is_symbol_name("answer") && statement_root.degree() == 1 {
            return Some(Suppose {
                requirements: vec![],
                resolution:   MarkedStatement::from(Arc::from(Statement::from(
                    (*statement.statement)
                        .clone()
                        .root_mut()
                        .pop_front()
                        .unwrap(),
                ))),
            });
        }

        match self {
            Self::Find(x) => {
                if statement_root.degree() != 2 ||
                    (!statement_root.data().is_symbol_name("==") &&
                        !statement_root.data().is_symbol_name("in"))
                {
                    return None;
                }

                if statement_root.front().unwrap() == x.statement.root() {
                    let is_known = tr(Term::with_symbol_name("is").unwrap()) /
                        statement_root.back().unwrap().deep_clone() /
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
                    if i.statement.root().check_truth() {
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
                    if new_states.is_empty() {
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
                    if new_states.is_empty() {
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
        false
    }
}

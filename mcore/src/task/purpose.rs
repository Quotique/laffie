use std::{fmt, rc::Rc, sync::Arc};

use trees::tr;

use crate::{
    rule::{RuleAttr, RulesEngine, SharedRule, Suppose},
    term::{symbol::Symbol, tree_utils::NodeMapping, Term, TermProps},
    utils::Dumper,
};

use super::{cache::TasksCache, frame::Frame, solution::MAX_LEVEL};

// TODO: remove
#[derive(Clone, Debug)]
pub enum SemanticError {
    UnexpectedWord(String),
    WorngArgCount(String),
}

pub enum Purpose {
    Find(TermProps),
    Proof(Frame),
    Transform(Frame),
}

impl fmt::Debug for Purpose {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Purpose::Find(s) => write!(f, "Find: {s:?}"),
            Purpose::Proof(s) => write!(f, "Proof: {s:?}"),
            Purpose::Transform(s) => write!(f, "Transform: {s:?}"),
        }
    }
}

impl fmt::Display for Purpose {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Purpose::Find(s) => write!(f, "Find: {s}"),
            Purpose::Proof(s) => write!(f, "Proof: {}", s[0]),
            Purpose::Transform(s) => write!(f, "Transform: {}", s[0]),
        }
    }
}

impl Purpose {
    pub fn try_from(
        value: Term,
        rules: Arc<RulesEngine>,
        dumper: Dumper,
        subtask_level: usize,
    ) -> Result<Self, SemanticError> {
        let (root, mut childs) = value.destruct();

        if root.data().is_symbol_name("find") {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(String::default()));
            }
            Ok(Self::Find(TermProps::from(Rc::new(Term::from(
                childs.pop_front().unwrap(),
            )))))
        } else if root.data().is_symbol_name("proof") {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(String::default()));
            }

            let mut frame = Frame::new(rules, dumper, subtask_level);
            // TODO: error Processing
            let _ = frame.add_condition(TermProps::from(Rc::new(Term::from(
                childs.pop_front().unwrap(),
            ))));
            Ok(Self::Proof(frame))
        } else if root.data().is_symbol_name("transform") {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(String::default()));
            }

            let mut frame = Frame::new(rules, dumper, subtask_level);
            // TODO: error Processing
            let _ = frame.add_condition(TermProps::from(Rc::new(Term::from(
                childs.pop_front().unwrap(),
            ))));
            Ok(Self::Transform(frame))
        } else {
            Err(SemanticError::UnexpectedWord(root.to_string()))
        }
    }

    pub fn is_answer(&self, term: &TermProps) -> Option<Suppose> {
        let term_root = term.term.root();

        if !self.is_transform() &&
            term_root.data().is_symbol_name("answer") &&
            term_root.degree() == 1
        {
            let mut resolution = TermProps::from(Rc::from(Term::from(
                (*term.term).clone().root_mut().pop_front().unwrap(),
            )));
            if let Some(parent) = term.parent {
                resolution = resolution.with_parent(parent);
            }
            return Some(Suppose {
                requirements: vec![],
                resolution,
            });
        }

        match self {
            Self::Find(x) => {
                if term_root.degree() != 2 ||
                    (!term_root.data().is_symbol_name("==") &&
                        !term_root.data().is_symbol_name("in"))
                {
                    return None;
                }

                if term_root.front().unwrap() == x.term.root() {
                    let is_known = tr(Symbol::with_func_symbol("is")) /
                        term_root.back().unwrap().deep_clone() /
                        tr(Symbol::with_func_symbol("known"));

                    return Some(Suppose {
                        requirements: vec![Rc::new(Term::from(is_known))],
                        resolution:   term.clone(),
                    });
                }
                None
            }
            Self::Proof(x) => {
                for i in x.iter() {
                    if term_root == i.term.root() {
                        return Some(Suppose {
                            requirements: vec![],
                            resolution:   term.clone(),
                        });
                    }
                    if i.term.root().check_truth().is_true() {
                        return Some(Suppose {
                            requirements: vec![],
                            resolution:   i.clone().without_parents(),
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
                            resolution:   x.last().unwrap().clone().without_parents(),
                        });
                    }
                }
                None
            }
        }
    }

    pub fn prepare_purpose(
        &mut self,
        level: usize,
        local_rules: Vec<SharedRule>,
        main_frame: &Frame,
        purpose: &TermProps,
        cache: Arc<TasksCache>,
    ) {
        match self {
            Self::Find(_) => {}
            Self::Proof(x) => {
                while let Ok(index) = x.pick_condition() {
                    if x[index].weight > level {
                        return;
                    }

                    if let Some(simplified) = x.transform(index, cache.clone()) {
                        x[index].replaced = true;
                        x.add_condition(simplified).unwrap();
                        continue;
                    } else {
                        x[index].simplified = true;
                    }

                    let new_states = main_frame.next_term_with_term(
                        &local_rules,
                        &mut x[index],
                        purpose,
                        |rule| rule.contains_attribute(&RuleAttr::Equivalence),
                        cache.clone(),
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
                    let new_states = main_frame.next_term_with_term(
                        &local_rules,
                        &mut x[index],
                        purpose,
                        |_| true,
                        cache.clone(),
                    );

                    if new_states.is_empty() {
                        x[index].weight += 1;
                    }
                    for s in new_states {
                        if x.contains(&s.term) {
                            continue;
                        }

                        if x.add_condition(s).is_ok() {
                            x[index].weight = MAX_LEVEL + 1;
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn is_transform(&self) -> bool {
        if let Purpose::Transform(_) = self {
            return true;
        }
        false
    }
}

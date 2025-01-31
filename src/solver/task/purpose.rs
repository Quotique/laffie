use std::{convert::TryFrom, fmt, rc::Rc};

use crate::term::{Term, TermProps};

// TODO: remove
#[derive(Clone, Debug)]
pub enum SemanticError {
    UnexpectedWord(String),
    WorngArgCount(String),
}

pub enum Purpose {
    Find(TermProps),
    Proof(TermProps),
    Transform(TermProps),
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
            Purpose::Proof(s) => write!(f, "Proof: {s}"),
            Purpose::Transform(s) => write!(f, "Transform: {s}"),
        }
    }
}

impl TryFrom<Term> for Purpose {
    type Error = SemanticError;

    fn try_from(value: Term) -> Result<Self, Self::Error> {
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
            // TODO: error Processing
            Ok(Self::Proof(TermProps::from(Rc::new(Term::from(
                childs.pop_front().unwrap(),
            )))))
        } else if root.data().is_symbol_name("transform") {
            if childs.degree() != 1 {
                return Err(SemanticError::WorngArgCount(String::default()));
            }

            // TODO: error Processing
            Ok(Self::Transform(TermProps::from(Rc::new(Term::from(
                childs.pop_front().unwrap(),
            )))))
        } else {
            Err(SemanticError::UnexpectedWord(root.to_string()))
        }
    }
}

impl Purpose {
    pub fn is_transform(&self) -> bool {
        if let Purpose::Transform(_) = self {
            return true;
        }
        false
    }
}

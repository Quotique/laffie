use std::{convert::TryFrom, fmt};

use super::TermProps;
use crate::term::Term;

// TODO: remove
#[derive(Clone, Debug)]
pub enum SemanticError {
    UnexpectedWord(String),
    WorngArgCount(String),
}

#[derive(Clone)]
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

    fn try_from(mut value: Term) -> Result<Self, Self::Error> {
        let mut root = value.as_subterm_mut();
        if root.degree() != 1 {
            return Err(SemanticError::WorngArgCount(String::default()));
        }
        let mut term = TermProps::from(root.pop_first_arg().unwrap());
        term.filters.mark_purpose();

        match root.data().symbol() {
            Some(x) if x.as_str() == "find" => Ok(Self::Find(term)),
            Some(x) if x.as_str() == "proof" => Ok(Self::Proof(term)),
            Some(x) if x.as_str() == "transform" => Ok(Self::Transform(term)),
            Some(_) | None => Err(SemanticError::UnexpectedWord(value.to_string())),
        }
    }
}

impl Purpose {
    #[inline]
    pub fn term(&self) -> &TermProps {
        match self {
            Purpose::Find(s) => s,
            Purpose::Proof(s) => s,
            Purpose::Transform(s) => s,
        }
    }

    #[inline]
    pub fn is_transform(&self) -> bool {
        if let Purpose::Transform(_) = self {
            return true;
        }
        false
    }

    #[inline]
    pub fn is_proof(&self) -> bool {
        if let Purpose::Proof(_) = self {
            return true;
        }
        false
    }

    #[inline]
    pub fn is_find(&self) -> bool {
        if let Purpose::Find(_) = self {
            return true;
        }
        false
    }
}

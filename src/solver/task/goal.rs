use std::{convert::TryFrom, fmt};

use super::TermProps;
use crate::term::TermBuf;

// TODO: remove
#[derive(Clone, Debug)]
pub enum SemanticError {
    UnexpectedWord(String),
    WrongArgCount(String),
}

#[derive(Clone)]
pub enum Goal {
    Find(TermProps),
    Prove(TermProps),
    Transform(TermProps),
}

impl fmt::Debug for Goal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Goal::Find(s) => write!(f, "Find: {s:?}"),
            Goal::Prove(s) => write!(f, "Prove: {s:?}"),
            Goal::Transform(s) => write!(f, "Transform: {s:?}"),
        }
    }
}

impl fmt::Display for Goal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Goal::Find(s) => write!(f, "Find: {s}"),
            Goal::Prove(s) => write!(f, "Prove: {s}"),
            Goal::Transform(s) => write!(f, "Transform: {s}"),
        }
    }
}

impl TryFrom<TermBuf> for Goal {
    type Error = SemanticError;

    fn try_from(mut value: TermBuf) -> Result<Self, Self::Error> {
        let mut root = value.term_mut();
        if root.degree() != 1 {
            return Err(SemanticError::WrongArgCount(String::default()));
        }
        let mut term = TermProps::from(root.pop_first_arg().unwrap());
        term.filters.mark_goal();

        match root.data().symbol() {
            Some(x) if x.as_str() == "find" => Ok(Self::Find(term)),
            Some(x) if x.as_str() == "prove" => Ok(Self::Prove(term)),
            Some(x) if x.as_str() == "transform" => Ok(Self::Transform(term)),
            Some(_) | None => Err(SemanticError::UnexpectedWord(value.to_string())),
        }
    }
}

impl Goal {
    #[inline]
    pub fn term(&self) -> &TermProps {
        match self {
            Goal::Find(s) => s,
            Goal::Prove(s) => s,
            Goal::Transform(s) => s,
        }
    }

    #[inline]
    pub fn is_transform(&self) -> bool {
        if let Goal::Transform(_) = self {
            return true;
        }
        false
    }

    #[inline]
    pub fn is_prove(&self) -> bool {
        if let Goal::Prove(_) = self {
            return true;
        }
        false
    }

    #[inline]
    pub fn is_find(&self) -> bool {
        if let Goal::Find(_) = self {
            return true;
        }
        false
    }
}

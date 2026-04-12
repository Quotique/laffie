use std::{convert::TryFrom, fmt};

use super::TermProps;
use crate::term::TermBuf;

// TODO: remove
#[derive(Clone, Debug)]
pub enum SemanticError {
    UnexpectedWord(String),
    WrongArgCount(String),
}

#[derive(Clone, Debug)]
pub struct FindGoal {
    pub targets: Vec<TermBuf>,
    pub term:    TermProps,
}

#[derive(Clone)]
pub enum Goal {
    Find(FindGoal),
    Prove(TermProps),
    Transform(TermProps),
}

impl fmt::Debug for Goal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Goal::Find(g) => write!(f, "Find: {:?}", g.term),
            Goal::Prove(s) => write!(f, "Prove: {s:?}"),
            Goal::Transform(s) => write!(f, "Transform: {s:?}"),
        }
    }
}

impl fmt::Display for Goal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Goal::Find(g) => write!(f, "Find: {}", g.term),
            Goal::Prove(s) => write!(f, "Prove: {s}"),
            Goal::Transform(s) => write!(f, "Transform: {s}"),
        }
    }
}

impl TryFrom<TermBuf> for Goal {
    type Error = SemanticError;

    fn try_from(mut value: TermBuf) -> Result<Self, Self::Error> {
        let mut root = value.term_mut();
        let is_find = root.data().symbol().is_some_and(|s| s.as_str() == "find");

        if !is_find && root.degree() != 1 {
            return Err(SemanticError::WrongArgCount(String::default()));
        }
        if is_find && root.degree() == 0 {
            return Err(SemanticError::WrongArgCount(String::default()));
        }

        if is_find {
            let mut targets = Vec::with_capacity(root.degree());
            while let Some(arg) = root.pop_first_arg() {
                targets.push(arg);
            }
            let mut term = TermProps::from(targets[0].clone());
            term.filters.mark_goal();
            return Ok(Self::Find(FindGoal { targets, term }));
        }

        let mut term = TermProps::from(root.pop_first_arg().unwrap());
        term.filters.mark_goal();

        match root.data().symbol() {
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
            Goal::Find(g) => &g.term,
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

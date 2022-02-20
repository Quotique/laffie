#[macro_use]
extern crate log;

use std::fmt;

mod dir_loader;
mod grammar;
mod problem;
mod rule;
mod statement;
mod symbol;

pub use self::{
    dir_loader::DirectoryParser, grammar::ra, problem::ProblemParser, rule::RuleParser,
    statement::StatementParser, symbol::SymbolParser,
};

pub type Tree = trees::Tree<String>;
pub type Node = trees::Node<String>;

#[derive(Clone, Debug)]
pub enum SemanticError {
    UnexpectedWord(String),
    WorngArgCount(String),
    MissingWord(String),

    Other(String),
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UnexpectedWord(w) => write!(f, "Unexpected word {}", w),
            Self::WorngArgCount(e) => write!(f, "Arg count missmath: {}", e),
            Self::MissingWord(w) => write!(f, "Missing word {}", w),
            Self::Other(e) => write!(f, "Semantic error: {}", e),
        }
    }
}

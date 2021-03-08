use std::fmt;

#[cfg(test)]
use crate::statement::Statement;

mod grammar;
mod problem;
mod rule;
mod statement;
mod symbol;

pub use self::grammar::ra;

pub type Tree = trees::Tree<String>;
pub type Node = trees::Node<String>;

pub use self::{
    problem::ProblemParser, rule::RuleParser, statement::StatementParser, symbol::SymbolParser,
};

#[derive(Clone, Debug)]
pub enum SemanticError {
    UnexpectedWord(String),
    WorngArgCount(String),
    MissingWord(String),

    Other(String),
}

#[allow(dead_code)]
#[cfg(test)]
pub fn statement_with_params(text: &str) -> Statement {
    let states = ra::statements(text).unwrap();
    StatementParser::new(&states[0]).parse().unwrap()
}

#[allow(dead_code)]
#[cfg(test)]
pub fn statement_with_vars(text: &str) -> Statement {
    let states = ra::statements(text).unwrap();
    StatementParser::new(&states[0])
        .with_variables()
        .parse()
        .unwrap()
}

#[allow(dead_code)]
#[cfg(test)]
pub fn parse_problem(text: &str) -> crate::problem::Problem {
    let states = ra::problem(text).unwrap();
    ProblemParser::with(&states).parse().unwrap()
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

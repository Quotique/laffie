#[macro_use]
extern crate log;

mod dir_loader;
mod error;
mod py_term;
mod rule;
mod scope;
mod source;
mod symbol;
mod syntax;
mod task;

pub use self::{
    dir_loader::{DirectoryLoader, LoadReport},
    error::{LoadError, LoadFailure, ParserError},
    scope::Names,
};

/// One term, as a test or a tool writes it.
pub fn term(text: &str, names: Names) -> Result<solver::term::TermBuf, ParserError> {
    let expr = syntax::grammar::parse::one_term(text).map_err(ParserError::from)?;
    let src = source::Source::new(text);
    scope::TermScope::new(&src, names).term(&expr)
}

/// Every rule of one `rule { … }` block.
pub fn rule(text: &str) -> Result<Vec<solver::rule::Rule>, ParserError> {
    let parsed = syntax::grammar::parse::one_rule(text).map_err(ParserError::from)?;
    parsed.build(&source::Source::new(text))
}

/// One `task { … }` block.
pub fn task(text: &str) -> Result<solver::task::Task, ParserError> {
    let parsed = syntax::grammar::parse::one_task(text).map_err(ParserError::from)?;
    parsed.build(&source::Source::new(text))
}

/// One `symbol …` declaration, unregistered.
pub fn symbol(text: &str) -> Result<solver::term::SymbolProgram, ParserError> {
    let parsed = syntax::grammar::parse::one_symbol(text).map_err(ParserError::from)?;
    parsed.build(&source::Source::new(text))
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Location {
    pub row: usize,
    pub col: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_string_row_past_end_does_not_panic() {
        let text = "a\nb\nc";
        let error = ParserError {
            loc: Location { row: 999, col: 0 },
            msg: "boom".to_owned(),
        };
        let _ = error.error_string(text, None); // must not panic
    }
}

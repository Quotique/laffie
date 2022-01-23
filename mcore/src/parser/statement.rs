use eyre::Result;

use crate::statement::Statement;

use super::Node;

pub struct StatementParser<'a> {
    ast:      &'a Node,
    with_var: bool,
}

impl<'a> StatementParser<'a> {
    pub fn new(syntax_tree: &'a Node) -> Self {
        Self {
            ast:      syntax_tree,
            with_var: false,
        }
    }

    pub fn with_variables(mut self) -> Self {
        self.with_var = true;
        self
    }

    pub fn parse(self) -> Result<Statement> {
        Ok(if self.with_var {
            Statement::try_parse_statement(self.ast)?
        } else {
            Statement::try_parse_rule(self.ast)?
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser::ra, predefine::setup, statement::term::Term};

    use trees::tr;

    #[test]
    fn parser_test() {
        setup();

        let test = "a*x + b == 0";
        let states = ra::statements(test).unwrap();
        assert_eq!(states.len(), 1);

        let result = StatementParser::new(&states[0]).parse();
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            (tr(Term::with_symbol_name("==").unwrap()) /
                (tr(Term::with_symbol_name("+").unwrap()) /
                    (tr(Term::with_symbol_name("*").unwrap()) /
                        tr(Term::Param("a".parse().unwrap())) /
                        tr(Term::Param("x".parse().unwrap()))) /
                    tr(Term::Param("b".parse().unwrap()))) /
                tr(Term::Number(0.into())))
            .into()
        );
    }
}

use crate::{
    core::term::{parse_rule_node, parse_statement_node},
    statement::{ParamsMap, Statement},
};

use super::Node;

pub struct StatementParser<'a> {
    ast:      &'a Node,
    params:   Option<&'a mut ParamsMap>,
    with_var: bool,
}

impl<'a> StatementParser<'a> {
    pub fn new(syntax_tree: &'a Node) -> Self {
        Self {
            ast:      syntax_tree,
            params:   None,
            with_var: false,
        }
    }

    pub fn with_params(mut self, params: &'a mut ParamsMap) -> Self {
        self.params = Some(params);
        self
    }

    pub fn with_variables(mut self) -> Self {
        self.with_var = true;
        self
    }

    pub fn parse(self) -> Result<Statement, String> {
        let mut empty_params = ParamsMap::new();
        let params: &mut ParamsMap = self.params.unwrap_or(&mut empty_params);

        let mut params_count: u64 = *params.values().max().unwrap_or(&0);

        let root = if self.with_var {
            parse_statement_node(self.ast, params, &mut params_count)?
        } else {
            parse_rule_node(self.ast, params, &mut params_count)?
        };
        Ok(root.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::term::Term, parser::LangParser, predefine::setup};
    use trees::tr;

    #[test]
    fn parser_test() {
        setup();

        let test = "a*x + b == 0";
        let states = LangParser::new().parse(test).unwrap();
        assert_eq!(states.len(), 1);

        let result = StatementParser::new(&states[0]).parse();
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            (tr(Term::with_symbol_name("==").unwrap()) /
                (tr(Term::with_symbol_name("+").unwrap()) /
                    (tr(Term::with_symbol_name("*").unwrap()) /
                        tr(Term::Param(1)) /
                        tr(Term::Param(2))) /
                    tr(Term::Param(3))) /
                tr(Term::Number(0.into())))
            .into()
        );
    }
}

use std::{collections::HashMap, str::FromStr};

use solver::{
    NormLevel, from_decimal_str,
    term::{ArgList, Atom, Param, Substitute, Term, TermBuf, Variable, try_sym},
};

use crate::{Node, ParserError};

#[derive(Default)]
pub struct TermParser {
    params:          HashMap<Param, TermBuf>,
    with_var:        bool,
    last_arglist_id: u64,
}

impl TermParser {
    pub fn with_variables(mut self) -> Self {
        self.with_var = true;
        self
    }

    pub fn with_params(mut self, params: HashMap<Param, TermBuf>) -> Self {
        self.params = params;
        self
    }

    pub fn try_parse(&mut self, node: &Node) -> Result<TermBuf, ParserError> {
        let mut tree = self.try_parse_node(node)?;

        tree.substitute_iter(self.params.clone());
        Ok(tree.normalize(NormLevel::Off))
    }

    fn try_parse_node(&mut self, mut node: &Node) -> Result<TermBuf, ParserError> {
        let mut params = vec![];
        while node.data().symbol == "as" {
            params.push(
                Param::from_str(&node.back().unwrap().data().symbol)
                    .expect("unable to create param"),
            );
            node = node.front().unwrap();
        }

        let value = self.parse_term(node.data().symbol.as_str());
        let mut tree = TermBuf::from(value);

        if tree.term().data().symbol().is_some() {
            for child in node.iter() {
                let arg = self.try_parse_node(child)?;
                tree.term_mut().push_last_arg(arg);
            }
        } else if node.degree() != 0 {
            return Err(ParserError {
                loc: node.data().location.clone(),
                msg: format!("Node {} can't contains children!", node.data().symbol),
            });
        }
        for p in params {
            if self.params.insert(p.clone(), tree.clone()).is_some() {
                return Err(ParserError {
                    loc: node.data().location.clone(),
                    msg: format!("Multiple definition of param {p}"),
                });
            }
        }

        Ok(tree)
    }

    fn parse_term(&mut self, data: &str) -> Atom {
        if data == ".." {
            self.last_arglist_id += 1;
            Atom::ArgList(ArgList::from(self.last_arglist_id))
        } else if let Some(value) = from_decimal_str(data) {
            Atom::Number(value)
        } else if let Some(symbol) = try_sym(data) {
            Atom::Symbol(symbol)
        } else if self.with_var {
            Atom::Variable(Variable::from_str(data).expect("unable to create variable"))
        } else {
            Atom::Param(Param::from_str(data).expect("unable to create param"))
        }
    }
}

#[cfg(test)]
mod tests {
    use solver::term::param;

    use crate::lang;

    use super::*;

    #[test]
    fn parser_test() {
        let test = "a*x + b == 0";
        let states = lang::terms(test).unwrap();
        assert_eq!(states.len(), 1);

        let result = TermParser::default().try_parse(&states[0]);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            TermBuf::symbol("==")
                .arg(
                    TermBuf::symbol("+")
                        .arg(TermBuf::symbol("*").arg(param("a")).arg(param("x")))
                        .arg(param("b"))
                )
                .arg(TermBuf::zero())
        );
    }

    #[test]
    fn binds_test() {
        let test = "set(5, x as S) is known <=> set(S) is known";
        let states = lang::terms(test).unwrap();
        assert_eq!(states.len(), 1);

        let result = TermParser::default().try_parse(&states[0]);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            TermBuf::symbol("<=>")
                .arg(
                    TermBuf::symbol("is")
                        .arg(
                            TermBuf::symbol("set")
                                .arg(TermBuf::number(5))
                                .arg(param("x"))
                        )
                        .arg(TermBuf::symbol("known"))
                )
                .arg(
                    TermBuf::symbol("is")
                        .arg(TermBuf::symbol("set").arg(param("x")))
                        .arg(TermBuf::symbol("known"))
                )
        );
    }

    fn parse(text: &str) -> Option<TermBuf> {
        let states = lang::terms(text).ok()?;
        TermParser::default().try_parse(&states[0]).ok()
    }

    #[track_caller]
    fn same(text: &str, explicit: &str) {
        assert_eq!(parse(text), parse(explicit), "{text} vs {explicit}");
    }

    #[test]
    fn a_negative_literal_leads_a_product() {
        same("-2*x", "(-2)*x");
        same("-2*x*y", "((-2)*x)*y");
        same("-3/x*y", "((-3)/x)*y");
        same("-2*(5 - sqrt(29)) > 0", "(-2)*(5 - sqrt(29)) > 0");
        same("x + -2*x", "x + (-2)*x");
    }

    #[test]
    fn a_negative_literal_binds_weaker_than_power() {
        same("-2^2", "-1*(2^2)");
        same("x - 2^2", "x + -1*(2^2)");
        same("-x^2", "-1*(x^2)");
    }

    #[test]
    fn a_double_minus_does_not_parse() {
        assert_eq!(parse("--2*x"), None);
        assert_eq!(parse("x - -2"), None);
    }
}

use std::{collections::HashMap, str::FromStr};

use solver::{
    term::{Param, ParamsMapping, Placeholder, Symbol, Term, TermNode, Variable},
    Decimal,
};

use crate::{Node, ParserError};

#[derive(Default)]
pub struct TermParser {
    params:              HashMap<Param, Term>,
    with_var:            bool,
    last_placeholder_id: u64,
}

impl TermParser {
    pub fn with_variables(mut self) -> Self {
        self.with_var = true;
        self
    }

    pub fn with_params(mut self, params: HashMap<Param, Term>) -> Self {
        self.params = params;
        self
    }

    pub fn try_parse(&mut self, node: &Node) -> Result<Term, ParserError> {
        let mut tree = self.try_parse_node(node)?;

        tree.as_subterm_mut()
            .apply_param_map(&ParamsMapping::from_iter(self.params.clone()));
        Ok(tree.normalize(0.into()))
    }

    fn try_parse_node(&mut self, mut node: &Node) -> Result<Term, ParserError> {
        let mut params = vec![];
        while node.data().symbol == "as" {
            params.push(
                Param::from_str(&node.back().unwrap().data().symbol)
                    .expect("unable to create param"),
            );
            node = node.front().unwrap();
        }

        let value = self.parse_term(node.data().symbol.as_str());
        let mut tree = Term::from(value);

        if tree.as_subterm().data().symbol().is_some() {
            for child in node.iter() {
                let arg = self.try_parse_node(child)?;
                tree.as_subterm_mut().push_last_arg(arg);
            }
        } else if node.degree() != 0 {
            return Err(ParserError {
                loc: node.data().location.clone(),
                msg: format!("Node {} can't contains children!", &node.data().symbol),
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

    fn parse_term(&mut self, data: &str) -> TermNode {
        if data == ".." {
            self.last_placeholder_id += 1;
            TermNode::Placeholder(Placeholder::from(self.last_placeholder_id))
        } else if let Ok(value) = Decimal::from_str(data) {
            TermNode::Number(value)
        } else if let Some(symbol) = Symbol::by_name(data) {
            TermNode::Symbol(symbol)
        } else if self.with_var {
            TermNode::Variable(Variable::from_str(data).expect("unable to create variable"))
        } else {
            TermNode::Param(Param::from_str(data).expect("unable to create param"))
        }
    }
}

#[cfg(test)]
mod tests {
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
            Term::symbol("==")
                .with_child(
                    Term::symbol("+")
                        .with_child(
                            Term::symbol("*")
                                .with_child(Term::param("a"))
                                .with_child(Term::param("x"))
                        )
                        .with_child(Term::param("b"))
                )
                .with_child(Term::number(0))
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
            Term::symbol("<=>")
                .with_child(
                    Term::symbol("is")
                        .with_child(
                            Term::symbol("set")
                                .with_child(Term::number(5))
                                .with_child(Term::param("x"))
                        )
                        .with_child(Term::symbol("known"))
                )
                .with_child(
                    Term::symbol("is")
                        .with_child(Term::symbol("set").with_child(Term::param("x")))
                        .with_child(Term::symbol("known"))
                )
        );
    }
}

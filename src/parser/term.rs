use std::{collections::HashMap, str::FromStr};

use trees::tr;

use solver::{
    term::{FuncSymbol, NodePosition, Param, Placeholder, Symbol, SymbolTree, Term, Variable},
    Decimal,
};

use crate::{Node, ParserError};

#[derive(Clone, Copy)]
enum NodeType {
    Statement,
    Rule,
}

pub struct TermParser<'a> {
    ast:      &'a Node,
    with_var: bool,
}

impl<'a> TermParser<'a> {
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

    pub fn parse(self) -> Result<Term, ParserError> {
        let mut positions_map = Default::default();
        let tree = if self.with_var {
            Self::try_parse_impl(
                self.ast,
                NodeType::Statement,
                Default::default(),
                &mut positions_map,
                &mut 0,
            )?
        } else {
            Self::try_parse_impl(
                self.ast,
                NodeType::Rule,
                Default::default(),
                &mut positions_map,
                &mut 0,
            )?
        };
        Ok(Term::new(tree, positions_map).normalize(0.into()))
    }

    fn try_parse_impl(
        mut node: &Node,
        node_type: NodeType,
        node_position: NodePosition,
        positions_map: &mut HashMap<Param, NodePosition>,
        last_placeholder_id: &mut u64,
    ) -> Result<SymbolTree, ParserError> {
        while node.data().symbol == "as" {
            let param = Param::from_str(&node.back().unwrap().data().symbol)
                .expect("unable to create param");
            if positions_map
                .insert(param.clone(), node_position.clone())
                .is_some()
            {
                return Err(ParserError {
                    loc: node.data().location.clone(),
                    msg: format!("Multiple definition of param {param}"),
                });
            }

            node = node.front().unwrap();
        }

        let mut tree = tr(Self::parse_term(
            node.data().symbol.as_str(),
            &node_type,
            last_placeholder_id,
        ));
        if tree.root().data().func_symbol().is_some() {
            for (num, child) in node.iter().enumerate() {
                tree.push_back(Self::try_parse_impl(
                    child,
                    node_type,
                    node_position.clone().child(num),
                    positions_map,
                    last_placeholder_id,
                )?);
            }
        } else if node.degree() != 0 {
            return Err(ParserError {
                loc: node.data().location.clone(),
                msg: format!("Node {} can't contains children!", &node.data().symbol),
            });
        }

        Ok(tree)
    }

    fn parse_term(data: &str, node_type: &NodeType, last_placeholder_id: &mut u64) -> Symbol {
        if data == ".." {
            *last_placeholder_id += 1;
            Symbol::Placeholder(Placeholder::from(*last_placeholder_id))
        } else if let Ok(value) = Decimal::from_str(data) {
            Symbol::Number(value)
        } else if let Some(symbol) = FuncSymbol::by_name(data) {
            Symbol::FuncSymbol(symbol)
        } else {
            match node_type {
                NodeType::Rule => {
                    Symbol::Param(Param::from_str(data).expect("unable to create param"))
                }
                NodeType::Statement => {
                    Symbol::Variable(Variable::from_str(data).expect("unable to create variable"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use trees::tr;

    use solver::term::Symbol;

    use crate::lang;

    use super::*;

    #[test]
    fn parser_test() {
        let test = "a*x + b == 0";
        let states = lang::terms(test).unwrap();
        assert_eq!(states.len(), 1);

        let result = TermParser::new(&states[0]).parse();
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            (tr(Symbol::with_func_symbol("==")) /
                (tr(Symbol::with_func_symbol("+")) /
                    (tr(Symbol::with_func_symbol("*")) /
                        tr(Symbol::Param("a".parse().unwrap())) /
                        tr(Symbol::Param("x".parse().unwrap()))) /
                    tr(Symbol::Param("b".parse().unwrap()))) /
                tr(Symbol::Number(0.into())))
            .into()
        );
    }
}

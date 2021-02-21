use super::{Node, SemanticError};
use crate::statement::{Symbol, SymbolAttr, SymbolAttrValue};

use std::{collections::HashMap, str::FromStr};

pub struct SymbolParser<'a> {
    ast: &'a Node,
}

impl<'a> SymbolParser<'a> {
    pub fn new(syntax_tree: &'a Node) -> Self {
        Self { ast: syntax_tree }
    }

    pub fn parse(self) -> Result<Symbol, SemanticError> {
        if self.ast.data != "Declare" {
            return Err(SemanticError::UnexpectedWord(self.ast.data.clone()));
        }
        let mut symbol = Symbol {
            id:    0,
            name:  String::default(),
            attrs: HashMap::new(),
        };

        for sym_child in self.ast.iter() {
            if sym_child.data == "Symbol" {
                symbol.name = sym_child.first().unwrap().data.clone();
            } else if sym_child.data == "Attrs" {
                for attr in sym_child.iter() {
                    let a = Self::parse_attr(attr)?;
                    symbol.attrs.insert(a.0, a.1);
                }
            }
        }
        if symbol.name.is_empty() {
            return Err(SemanticError::MissingWord("Symbol".into()));
        }
        Ok(symbol)
    }

    fn parse_attr(data: &Node) -> Result<(SymbolAttr, SymbolAttrValue), SemanticError> {
        match data.data.as_str() {
            "infix" => {
                let c = data
                    .first()
                    .ok_or_else(|| SemanticError::Other("infix(w) argument is required!".into()))?;
                let w = u64::from_str(&c.data)
                    .map_err(|_| SemanticError::Other("Infix argument must be u64".into()))?;
                Ok((SymbolAttr::Infix, SymbolAttrValue::UInt(w)))
            }
            "display" => {
                let s = data
                    .first()
                    .ok_or_else(|| SemanticError::Other("display(s) argument is required!".into()))?
                    .data
                    .clone();
                Ok((SymbolAttr::Display, SymbolAttrValue::UStr(s)))
            }
            "associative" => Ok((SymbolAttr::Associative, SymbolAttrValue::None)),
            "commutative" => Ok((SymbolAttr::Commutative, SymbolAttrValue::None)),
            _ => Err(SemanticError::UnexpectedWord(data.data.clone())),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use parser::LangParser;

    #[test]
    fn parser_test() {
        let test_str = "symbol + { attr infix(10) }";
        let states = LangParser::new().parse(test_str).unwrap();

        let sym = SymbolParser::new(&states[0]).parse().unwrap();
        let mut expect_attr = HashMap::new();
        expect_attr.insert(SymbolAttr::Infix, SymbolAttrValue::UInt(10));
        assert_eq!(
            sym,
            Symbol {
                id:    0,
                name:  "+".into(),
                attrs: expect_attr,
            }
        );
    }
}

use std::str::FromStr;

use mcore::statement::{Symbol, SymbolAttr, SymbolAttrValue};

use super::{Node, SemanticError};

pub struct SymbolParser<'a> {
    ast: &'a Node,
}

impl<'a> SymbolParser<'a> {
    pub fn new(syntax_tree: &'a Node) -> Self {
        Self { ast: syntax_tree }
    }

    pub fn parse(self) -> Result<Symbol, SemanticError> {
        if self.ast.data() != "Declare" {
            return Err(SemanticError::UnexpectedWord(self.ast.data().clone()));
        }

        let mut builder = Symbol::builder();

        for sym_child in self.ast.iter() {
            if sym_child.data() == "Symbol" {
                builder.name(sym_child.front().unwrap().data().clone());
            } else if sym_child.data() == "Attrs" {
                for attr in sym_child.iter() {
                    let a = Self::parse_attr(attr)?;
                    builder.with_attr(a.0, a.1);
                }
            }
        }
        builder
            .build()
            .map_err(|e| SemanticError::MissingWord(e.to_string()))
    }

    fn parse_attr(data: &Node) -> Result<(SymbolAttr, SymbolAttrValue), SemanticError> {
        match data.data().as_str() {
            "infix" => {
                let c = data
                    .front()
                    .ok_or_else(|| SemanticError::Other("infix(w) argument is required!".into()))?;
                let w = u64::from_str(c.data())
                    .map_err(|_| SemanticError::Other("Infix argument must be u64".into()))?;
                Ok((SymbolAttr::Infix, SymbolAttrValue::UInt(w)))
            }
            "display" => {
                let s = data
                    .front()
                    .ok_or_else(|| SemanticError::Other("display(s) argument is required!".into()))?
                    .data()
                    .clone();
                Ok((SymbolAttr::Display, SymbolAttrValue::UStr(s)))
            }
            "associative" => Ok((SymbolAttr::Associative, SymbolAttrValue::None)),
            "commutative" => Ok((SymbolAttr::Commutative, SymbolAttrValue::None)),
            _ => Err(SemanticError::UnexpectedWord(data.data().clone())),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::ra;

    use super::*;

    #[test]
    fn parser_test() {
        let test_str = "symbol + { attr infix(10) }";
        let states = ra::symbol(test_str).unwrap();

        let sym = SymbolParser::new(&states).parse().unwrap();
        assert_eq!(
            sym,
            Symbol::builder()
                .name("+")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(10))
                .build()
                .unwrap()
        );
    }
}

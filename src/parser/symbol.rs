use std::str::FromStr;

use solver::term::{SymbolAttr, SymbolAttrValue, SymbolProgram};

use super::{Node, ParserError};

pub struct SymbolParser<'a> {
    ast: &'a Node,
}

impl<'a> SymbolParser<'a> {
    pub fn new(syntax_tree: &'a Node) -> Self {
        Self { ast: syntax_tree }
    }

    pub fn parse(self) -> Result<SymbolProgram, ParserError> {
        if self.ast.data().symbol != "Declare" {
            return Err(ParserError {
                loc: self.ast.data().location.clone(),
                msg: "expected 'symbol'".to_owned(),
            });
        }

        let mut symbol_program = SymbolProgram::default();

        for sym_child in self.ast.iter() {
            if sym_child.data().symbol == "Symbol" {
                symbol_program.name = sym_child.front().unwrap().data().symbol.clone();
            } else if sym_child.data().symbol == "Attrs" {
                for attr in sym_child.iter() {
                    let a = Self::parse_attr(attr)?;
                    symbol_program.attrs.insert(a.0, a.1);
                }
            }
        }
        if symbol_program.name.is_empty() {
            Err(ParserError {
                loc: self.ast.data().location.clone(),
                msg: "symbol name is mandatory".into(),
            })
        } else {
            Ok(symbol_program)
        }
    }

    fn parse_attr(data: &Node) -> Result<(SymbolAttr, SymbolAttrValue), ParserError> {
        match data.data().symbol.as_str() {
            "infix" => {
                let c = data.front().ok_or_else(|| ParserError {
                    loc: data.data().location.clone(),
                    msg: "infix(number) argument is required!".to_owned(),
                })?;
                let w = u64::from_str(&c.data().symbol).map_err(|_| ParserError {
                    loc: data.data().location.clone(),
                    msg: "infix argument must be u64".to_owned(),
                })?;
                Ok((SymbolAttr::Infix, SymbolAttrValue::UInt(w)))
            }
            "display" => {
                let s = data
                    .front()
                    .ok_or_else(|| ParserError {
                        loc: data.data().location.clone(),
                        msg: "display(s) argument is required!".to_owned(),
                    })?
                    .data();
                Ok((SymbolAttr::Display, SymbolAttrValue::UStr(s.symbol.clone())))
            }
            "associative" => Ok((SymbolAttr::Associative, SymbolAttrValue::None)),
            "commutative" => Ok((SymbolAttr::Commutative, SymbolAttrValue::None)),
            _ => Err(ParserError {
                loc: data.data().location.clone(),
                msg: "unknown attribute".to_owned(),
            }),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::lang;

    use super::*;

    #[test]
    fn parser_test() {
        let test_str = "symbol + { attr infix(10) }";
        let states = lang::symbol(test_str).unwrap();

        let sym = SymbolParser::new(&states).parse().unwrap();
        assert_eq!(sym.name, "+");
        assert_eq!(
            sym.attrs.get(&SymbolAttr::Infix),
            Some(&SymbolAttrValue::UInt(10))
        );
    }
}

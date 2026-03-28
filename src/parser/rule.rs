use std::str::FromStr;

use solver::{
    CompactString,
    rule::{Rule, RuleAttr, RuleAttrValue, RuleBuilder},
    term::Symbol,
};

use crate::ParserError;

use super::{Node, Tree, term::TermParser};

pub struct RuleParser<'a> {
    syntax_tree: &'a Tree,
    func_symbol: Symbol,
}

impl<'a> From<&'a Tree> for RuleParser<'a> {
    fn from(syntax_tree: &'a Tree) -> Self {
        Self {
            syntax_tree,
            func_symbol: Default::default(),
        }
    }
}

impl<'a> RuleParser<'a> {
    pub fn with_func_symbol(mut self, func_symbol: Symbol) -> Self {
        self.func_symbol = func_symbol;
        self
    }

    pub fn parse(self) -> Result<Vec<Rule>, ParserError> {
        if self.syntax_tree.root().data().symbol != "Rule" {
            return Err(ParserError {
                loc: self.syntax_tree.root().data().location.clone(),
                msg: "expected 'rule'".to_owned(),
            });
        }
        let mut builder = RuleBuilder::default();
        let mut parser = TermParser::default();

        for child in self.syntax_tree.iter() {
            match child.data().symbol.as_str() {
                "=>" | "<=>" => {
                    builder =
                        builder
                            .with_term(parser.try_parse(child)?)
                            .map_err(|e| ParserError {
                                loc: child.data().location.clone(),
                                msg: e.to_string(),
                            })?;
                }
                "Predicates" => {
                    for req in child.iter() {
                        builder = builder.with_require(parser.try_parse(req)?)
                    }
                }
                "Attributes" => {
                    for attr in child.iter() {
                        let (attr, value) = RuleParser::parse_attribute(attr)?;
                        builder = builder.with_attribute(attr, value)
                    }
                }
                _ => {
                    error!("{:?}", child.data().symbol);
                    return Err(ParserError {
                        loc: child.data().location.clone(),
                        msg: "unexpected word".to_owned(),
                    });
                }
            }
        }

        builder
            .with_func_symbol(self.func_symbol)
            .build()
            .map_err(|e| ParserError {
                loc: self.syntax_tree.data().location.clone(),
                msg: e.to_string(),
            })
    }

    fn parse_attribute(attr: &Node) -> Result<(RuleAttr, RuleAttrValue), ParserError> {
        match attr.data().symbol.as_str() {
            "subtree" | "equivalence" | "replace" => Ok((
                RuleAttr::from_str(attr.data().symbol.as_str()).unwrap(),
                RuleAttrValue::None,
            )),
            "level" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }
                let data = attr.front().unwrap().data();

                Ok((
                    RuleAttr::Level,
                    RuleAttrValue::UInt(data.symbol.parse::<u64>().map_err(|_| ParserError {
                        loc: data.location.clone(),
                        msg: "must be u64".to_owned(),
                    })?),
                ))
            }
            "goal" | "zero" | "one" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }

                let goal = TermParser::default().try_parse(attr.front().unwrap())?;
                Ok((
                    RuleAttr::from_str(attr.data().symbol.as_str()).unwrap(),
                    RuleAttrValue::Goal(goal),
                ))
            }
            "id" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }
                let data = attr.front().unwrap().data();

                Ok((
                    RuleAttr::from_str(attr.data().symbol.as_str()).unwrap(),
                    RuleAttrValue::Str(CompactString::from(data.symbol.as_str())),
                ))
            }
            "block" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }
                let data = attr.front().unwrap().data();

                Ok((
                    RuleAttr::from_str(attr.data().symbol.as_str()).unwrap(),
                    RuleAttrValue::Str(CompactString::from(data.symbol.as_str())),
                ))
            }
            "normalize" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }
                let data = attr.front().unwrap().data();

                Ok((
                    RuleAttr::Normalize,
                    RuleAttrValue::UInt(data.symbol.parse::<u64>().map_err(|_| ParserError {
                        loc: data.location.clone(),
                        msg: "must be u64".to_owned(),
                    })?),
                ))
            }
            _ => Err(ParserError {
                loc: attr.data().location.clone(),
                msg: "unknown attribute".to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use solver::{
        rule::{RuleAttr, RuleAttrValue},
        term::{TermBuf, param},
    };

    use crate::lang;

    use super::*;

    #[test]
    fn rule_parse_test() {
        let test = r#"rule {
                        attr replace,level(1);
                        a + x == 0 => x == -a;
                        a!=0;
                      }"#;

        let states = lang::lang_rule(test).unwrap();
        let result = RuleParser::from(&states).parse();
        assert!(result.is_ok());

        let mut rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        let rule = rules.pop().unwrap();

        assert_eq!(
            rule.pattern_node().to_owned(),
            TermBuf::symbol("==")
                .arg(TermBuf::symbol("+").arg(param("a")).arg(param("x")))
                .arg(TermBuf::number(0))
        );

        assert_eq!(
            rule.replace_node().to_owned(),
            TermBuf::symbol("==").arg(param("x")).arg(
                TermBuf::symbol("*")
                    .arg(TermBuf::number(-1))
                    .arg(param("a"))
            )
        );
        assert_eq!(rule.requirements.len(), 1);
        assert_eq!(
            rule.requirements[0],
            TermBuf::symbol("!=")
                .arg(param("a"))
                .arg(TermBuf::number(0))
        );

        assert_eq!(rule.attrs.len(), 2);

        assert!(rule.contains_attribute(&RuleAttr::Replace));
        assert_eq!(
            rule.attribute(&RuleAttr::Level).collect::<Vec<_>>(),
            vec![&RuleAttrValue::UInt(1)]
        );
    }

    #[test]
    fn rule_parse_2_test() {
        let test = r#"rule {
                    attr level(0),goal(prove(a/b is known));
                    a/b is known <=> true;
                    a is known,
                    b is known
                }"#;

        let states = lang::lang_rule(test).unwrap();
        let result = RuleParser::from(&states).parse();
        assert!(result.is_ok());

        let mut rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        let rule = rules.pop().unwrap();

        assert_eq!(
            rule.pattern_node().to_owned(),
            TermBuf::symbol("is")
                .arg(TermBuf::symbol("/").arg(param("a")).arg(param("b")))
                .arg(TermBuf::symbol("known"))
        );

        assert_eq!(rule.replace_node().to_owned(), TermBuf::symbol("true"));
        assert_eq!(rule.requirements.len(), 2);
        assert_eq!(
            rule.requirements[0],
            TermBuf::symbol("is")
                .arg(param("a"))
                .arg(TermBuf::symbol("known"))
        );
        assert_eq!(
            rule.requirements[1],
            TermBuf::symbol("is")
                .arg(param("b"))
                .arg(TermBuf::symbol("known"))
        );

        assert_eq!(rule.attrs.len(), 3);

        assert!(rule.contains_attribute(&RuleAttr::Equivalence));
        assert_eq!(
            rule.attribute(&RuleAttr::Level).collect::<Vec<_>>(),
            vec![&RuleAttrValue::UInt(0)]
        );
    }

    #[test]
    fn arg_reduction_test() {
        let test = r#"rule {
                        attr replace,level(1),one(a),zero(b);
                        a * x + b == 0 => x == -b / a;
                        a!=0;
                      }"#;

        let states = lang::lang_rule(test).unwrap();
        let result = RuleParser::from(&states).parse();
        assert!(result.is_ok());

        let rules = result.unwrap();
        assert_eq!(rules.len(), 3);
    }

    #[test]
    fn attr_parse_test() {
        let test = r#"rule {
            attr replace,level(1),id(move_left),block(hello),block(world),normalize(4);
            a == b => a - b == 0;
            b != 0;
        }"#;

        let states = lang::lang_rule(test).map_err(|e| println!("{e}")).unwrap();
        let result = RuleParser::from(&states).parse();
        assert!(result.is_ok());

        let mut rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        let rule = rules.pop().unwrap();
        assert_eq!(
            rule.attribute(&RuleAttr::Id).collect::<Vec<_>>(),
            vec![&RuleAttrValue::Str("move_left".into())]
        );
        assert_eq!(
            rule.attribute(&RuleAttr::Block).collect::<Vec<_>>(),
            vec![
                &RuleAttrValue::Str("hello".into()),
                &RuleAttrValue::Str("world".into())
            ]
        );
        assert_eq!(
            rule.attribute(&RuleAttr::Normalize).collect::<Vec<_>>(),
            vec![&RuleAttrValue::UInt(4)]
        );
    }
}

use std::{str::FromStr, sync::Arc};

use solver::{
    rule::{Rule, RuleAttr, RuleAttrValue, RuleBuilder},
    symbol::FuncSymbol,
    CompactString,
};

use crate::ParserError;

use super::{term::TermParser, Node, Tree};

pub struct RuleParser<'a> {
    syntax_tree: &'a Tree,
    func_symbol: Arc<FuncSymbol>,
}

impl<'a> RuleParser<'a> {
    pub fn with(syntax_tree: &'a Tree) -> Self {
        Self {
            syntax_tree,
            func_symbol: Default::default(),
        }
    }

    pub fn with_func_symbol(mut self, func_symbol: Arc<FuncSymbol>) -> Self {
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

        for child in self.syntax_tree.iter() {
            match child.data().symbol.as_str() {
                "=>" | "<=>" => {
                    builder = builder
                        .with_term(TermParser::new(child).parse()?)
                        .map_err(|e| ParserError {
                            loc: child.data().location.clone(),
                            msg: e.to_string(),
                        })?;
                }
                "Predicates" => {
                    for req in child.iter() {
                        builder = builder.with_require(TermParser::new(req).parse()?)
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
            "purpose" | "zero" | "one" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }

                let purpose = TermParser::new(attr.front().unwrap()).parse()?;
                Ok((
                    RuleAttr::from_str(attr.data().symbol.as_str()).unwrap(),
                    RuleAttrValue::Target(purpose),
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
        term::Term,
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
        let result = RuleParser::with(&states).parse();
        assert!(result.is_ok());

        let mut rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        let rule = rules.pop().unwrap();

        assert_eq!(
            rule.pattern_node().deep_clone(),
            Term::func("==")
                .with_child(
                    Term::func("+")
                        .with_child(Term::param("a"))
                        .with_child(Term::param("x"))
                )
                .with_child(Term::number(0))
        );

        assert_eq!(
            rule.replace_node().deep_clone(),
            Term::func("==").with_child(Term::param("x")).with_child(
                Term::func("*")
                    .with_child(Term::number(-1))
                    .with_child(Term::param("a"))
            )
        );
        assert_eq!(rule.requirements.len(), 1);
        assert_eq!(
            rule.requirements[0],
            Term::func("!=")
                .with_child(Term::param("a"))
                .with_child(Term::number(0))
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
                    attr level(0),purpose(proof(a/b is known));
                    a/b is known <=> true;
                    a is known,
                    b is known
                }"#;

        let states = lang::lang_rule(test).unwrap();
        let result = RuleParser::with(&states).parse();
        assert!(result.is_ok());

        let mut rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        let rule = rules.pop().unwrap();

        assert_eq!(
            rule.pattern_node().deep_clone(),
            Term::func("is")
                .with_child(
                    Term::func("/")
                        .with_child(Term::param("a"))
                        .with_child(Term::param("b"))
                )
                .with_child(Term::func("known"))
        );

        assert_eq!(rule.replace_node().deep_clone(), Term::func("true"));
        assert_eq!(rule.requirements.len(), 2);
        assert_eq!(
            rule.requirements[0],
            Term::func("is")
                .with_child(Term::param("a"))
                .with_child(Term::func("known"))
        );
        assert_eq!(
            rule.requirements[1],
            Term::func("is")
                .with_child(Term::param("b"))
                .with_child(Term::func("known"))
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
        let result = RuleParser::with(&states).parse();
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
        let result = RuleParser::with(&states).parse();
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

use super::{statement::StatementParser, Node, SemanticError, Tree};
use crate::rule::{Rule, RuleAttr, RuleAttrValue, RuleBuilder};
use std::str::FromStr;

pub struct RuleParser<'a> {
    syntax_tree: &'a Tree,
    symbol_id:   u64,
}

impl<'a> RuleParser<'a> {
    pub fn with(syntax_tree: &'a Tree) -> Self {
        Self {
            syntax_tree,
            symbol_id: 0,
        }
    }

    pub fn with_symbol_id(mut self, symbol_id: u64) -> Self {
        self.symbol_id = symbol_id;
        self
    }

    pub fn parse(self) -> Result<Vec<Rule>, SemanticError> {
        if self.syntax_tree.root().data() != "Rule" {
            return Err(SemanticError::UnexpectedWord(
                self.syntax_tree.root().data().clone(),
            ));
        }
        let mut builder = RuleBuilder::new();

        for child in self.syntax_tree.iter() {
            match child.data().as_str() {
                "=>" | "<=>" => {
                    builder = builder
                        .with_statement(
                            StatementParser::new(child)
                                .parse()
                                .map_err(|e| SemanticError::Other(e.to_string()))?,
                        )
                        .map_err(|e| SemanticError::Other(e.to_string()))?;
                }
                "Predicates" => {
                    for req in child.iter() {
                        builder = builder.with_require(
                            StatementParser::new(req)
                                .parse()
                                .map_err(|e| SemanticError::Other(e.to_string()))?,
                        )
                    }
                }
                "Attributes" => {
                    for attr in child.iter() {
                        let (attr, value) = RuleParser::parse_attribute(attr)?;
                        builder = builder.with_attribute(attr, value)
                    }
                }
                _ => {
                    error!("{:?}", child);
                    return Err(SemanticError::UnexpectedWord(child.data().clone()));
                }
            }
        }

        builder
            .with_symbol_id(self.symbol_id)
            .build()
            .map_err(|e| SemanticError::Other(e.to_string()))
    }

    fn parse_attribute(attr: &Node) -> Result<(RuleAttr, RuleAttrValue), SemanticError> {
        match attr.data().as_str() {
            "subtree" | "equivalence" | "replace" => Ok((
                RuleAttr::from_str(attr.data().as_str()).unwrap(),
                RuleAttrValue::None,
            )),
            "level" => {
                if attr.degree() != 1 {
                    return Err(SemanticError::WorngArgCount(format!(
                        "Wrong target arguments count: {} expect 1",
                        attr.degree()
                    )));
                }
                Ok((
                    RuleAttr::Level,
                    RuleAttrValue::UInt(attr.front().unwrap().data().parse::<u64>().map_err(
                        |_| SemanticError::UnexpectedWord(attr.front().unwrap().data().clone()),
                    )?),
                ))
            }
            "problem_target" | "zero" | "one" => {
                if attr.degree() != 1 {
                    return Err(SemanticError::WorngArgCount(format!(
                        "Wrong target arguments count: {} expect 1",
                        attr.degree()
                    )));
                }

                let target = StatementParser::new(attr.front().unwrap())
                    .parse()
                    .map_err(|e| SemanticError::Other(e.to_string()))?;
                Ok((
                    RuleAttr::from_str(attr.data().as_str()).unwrap(),
                    RuleAttrValue::Target(target),
                ))
            }
            _ => Err(SemanticError::UnexpectedWord(attr.data().clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parser::ra,
        predefine::setup,
        rule::{RuleAttr, RuleAttrValue},
        statement::term::Term,
    };

    use trees::tr;

    #[test]
    fn rule_parse_test() {
        setup();
        let test = r#"rule {
                        attr replace,level(1);
                        a + x == 0 => x == -a;
                        a!=0;
                      }"#;

        let states = ra::lang_rule(test).unwrap();
        let result = RuleParser::with(&states).parse();
        assert!(result.is_ok());

        let mut rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        let rule = rules.pop().unwrap();

        assert_eq!(
            rule.pattern_node().deep_clone(),
            (tr(Term::with_symbol_name("==").unwrap()) /
                (tr(Term::with_symbol_name("+").unwrap()) /
                    tr(Term::Param("a".parse().unwrap())) /
                    tr(Term::Param("x".parse().unwrap()))) /
                tr(Term::Number(0.into())))
        );

        assert_eq!(
            rule.replace_node().deep_clone(),
            (tr(Term::with_symbol_name("==").unwrap()) /
                tr(Term::Param("x".parse().unwrap())) /
                (tr(Term::with_symbol_name("-").unwrap()) /
                    tr(Term::Param("a".parse().unwrap()))))
        );
        assert_eq!(rule.requirements.len(), 1);
        assert_eq!(
            rule.requirements[0],
            (tr(Term::with_symbol_name("!=").unwrap()) /
                tr(Term::Param("a".parse().unwrap())) /
                tr(Term::Number(0.into())))
            .into()
        );

        assert_eq!(rule.attrs.len(), 3);

        assert!(rule.attribute(&RuleAttr::Replace).is_some());
        assert_eq!(
            rule.attribute(&RuleAttr::Level),
            Some(&RuleAttrValue::UInt(1))
        );
    }

    #[test]
    fn rule_parse_2_test() {
        setup();
        let test = r#"rule {
					attr level(0),problem_target(proof(a/b is known));
					a/b is known <=> true;
					a is known,
					b is known
				}"#;

        let states = ra::lang_rule(test).unwrap();
        let result = RuleParser::with(&states).parse();
        assert!(result.is_ok());

        let mut rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        let rule = rules.pop().unwrap();

        assert_eq!(
            rule.pattern_node().deep_clone(),
            (tr(Term::with_symbol_name("is").unwrap()) /
                (tr(Term::with_symbol_name("/").unwrap()) /
                    tr(Term::Param("a".parse().unwrap())) /
                    tr(Term::Param("b".parse().unwrap()))) /
                tr(Term::with_symbol_name("known").unwrap()))
        );

        assert_eq!(
            rule.replace_node().deep_clone(),
            (tr(Term::with_symbol_name("true").unwrap()))
        );
        assert_eq!(rule.requirements.len(), 2);
        assert_eq!(
            rule.requirements[0],
            (tr(Term::with_symbol_name("is").unwrap()) /
                tr(Term::Param("a".parse().unwrap())) /
                tr(Term::with_symbol_name("known").unwrap()))
            .into()
        );
        assert_eq!(
            rule.requirements[1],
            (tr(Term::with_symbol_name("is").unwrap()) /
                tr(Term::Param("b".parse().unwrap())) /
                tr(Term::with_symbol_name("known").unwrap()))
            .into()
        );

        assert_eq!(rule.attrs.len(), 4);

        assert!(rule.attribute(&RuleAttr::Subtree).is_some());
        assert!(rule.attribute(&RuleAttr::Equivalence).is_some());
        assert_eq!(
            rule.attribute(&RuleAttr::Level),
            Some(&RuleAttrValue::UInt(0))
        );
    }

    #[test]
    fn arg_reduction_test() {
        setup();
        let test = r#"rule {
                        attr replace,level(1),one(a),zero(b);
                        a * x + b == 0 => x == -b / a;
                        a!=0;
                      }"#;

        let states = ra::lang_rule(test).unwrap();
        let result = RuleParser::with(&states).parse();
        assert!(result.is_ok());

        let rules = result.unwrap();
        assert_eq!(rules.len(), 3);
    }
}

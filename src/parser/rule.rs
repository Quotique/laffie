use super::{statement::StatementParser, Node, SemanticError, Tree};
use crate::{
    rule::{Rule, RuleAttr, RuleAttrValue, RuleBuilder},
    statement::ParamsMap,
};

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

    pub fn parse(self) -> Result<Rule, SemanticError> {
        if self.syntax_tree.root().data != "Rule" {
            return Err(SemanticError::UnexpectedWord(
                self.syntax_tree.root().data.clone(),
            ));
        }
        let mut builder = RuleBuilder::new();
        let mut params = ParamsMap::new();

        for child in self.syntax_tree.iter() {
            match child.data.as_str() {
                "=>" | "<=>" => {
                    builder = builder
                        .with_statement(
                            StatementParser::new(child)
                                .with_params(&mut params)
                                .parse()
                                .map_err(SemanticError::Other)?,
                        )
                        .map_err(|e| SemanticError::Other(e.to_string()))?;
                }
                "Predicates" => {
                    for req in child.iter() {
                        builder = builder.with_require(
                            StatementParser::new(req)
                                .with_params(&mut params)
                                .parse()
                                .map_err(SemanticError::Other)?,
                        )
                    }
                }
                "Attributes" => {
                    for attr in child.iter() {
                        let (attr, value) = RuleParser::parse_attribute(attr, &mut params)?;
                        builder = builder.with_attribute(attr, value)
                    }
                }
                _ => return Err(SemanticError::UnexpectedWord(child.data.clone())),
            }
        }

        builder
            .with_symbol_id(self.symbol_id)
            .build()
            .map_err(|e| SemanticError::Other(e.to_string()))
    }

    fn parse_attribute(
        attr: &Node,
        params: &mut ParamsMap,
    ) -> Result<(RuleAttr, RuleAttrValue), SemanticError> {
        match attr.data.as_str() {
            "subtree" => Ok((RuleAttr::Subtree, RuleAttrValue::None)),
            "equivalence" => Ok((RuleAttr::Equivalence, RuleAttrValue::None)),
            "replace" => Ok((RuleAttr::Replace, RuleAttrValue::None)),
            "level" => {
                if attr.degree() != 1 {
                    return Err(SemanticError::WorngArgCount(format!(
                        "Wrong target arguments count: {} expect 1",
                        attr.degree()
                    )));
                }
                Ok((
                    RuleAttr::Level,
                    RuleAttrValue::UInt(attr.first().unwrap().data.parse::<u64>().map_err(
                        |_| SemanticError::UnexpectedWord(attr.first().unwrap().data.clone()),
                    )?),
                ))
            }
            "problem_target" => {
                if attr.degree() != 1 {
                    return Err(SemanticError::WorngArgCount(format!(
                        "Wrong target arguments count: {} expect 1",
                        attr.degree()
                    )));
                }

                let target = StatementParser::new(attr.first().unwrap())
                    .with_params(params)
                    .parse()
                    .map_err(SemanticError::Other)?;
                Ok((RuleAttr::Target, RuleAttrValue::Target(target)))
            }
            _ => Err(SemanticError::UnexpectedWord(attr.data.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::term::Term,
        parser::LangParser,
        predefine::setup,
        rule::{RuleAttr, RuleAttrValue},
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

        let states = LangParser::new().parse(test).unwrap();
        assert_eq!(states.len(), 1);

        let result = RuleParser::with(&states[0]).parse();
        assert!(result.is_ok());

        let rule = result.unwrap();
        assert_eq!(
            rule.pattern,
            (tr(Term::with_symbol_name("==").unwrap()) /
                (tr(Term::with_symbol_name("+").unwrap()) /
                    tr(Term::Param(1)) /
                    tr(Term::Param(2))) /
                tr(Term::Number(0.into())))
            .into()
        );

        assert_eq!(
            rule.replace,
            (tr(Term::with_symbol_name("==").unwrap()) /
                tr(Term::Param(2)) /
                (tr(Term::with_symbol_name("-").unwrap()) / tr(Term::Param(1))))
            .into()
        );
        assert_eq!(rule.requirements.len(), 1);
        assert_eq!(
            rule.requirements[0],
            (tr(Term::with_symbol_name("!=").unwrap()) /
                tr(Term::Param(1)) /
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
}

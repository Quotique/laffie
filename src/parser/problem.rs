use super::{statement::StatementParser, SemanticError, Tree};
use crate::{
    problem::{Problem, ProblemBuilder},
    statement::{MarkedStatement, ParamsMap},
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

pub struct ProblemParser<'a> {
    syntax_tree: &'a Tree,
}

impl<'a> ProblemParser<'a> {
    pub fn with(syntax_tree: &'a Tree) -> Self {
        Self { syntax_tree }
    }

    pub fn parse(self) -> Result<Problem, SemanticError> {
        if self.syntax_tree.root().data() != "Problem" {
            return Err(SemanticError::UnexpectedWord(
                self.syntax_tree.root().data().clone(),
            ));
        }
        let mut hasher = DefaultHasher::new();
        self.syntax_tree.root().hash(&mut hasher);
        let hash = hasher.finish();
        let mut params = ParamsMap::new();

        let mut builder = ProblemBuilder::new().with_id(hash);

        for child in self.syntax_tree.iter() {
            if child.data() == "Target" {
                if child.degree() != 1 {
                    return Err(SemanticError::WorngArgCount(format!(
                        "target: expect 1 found {}",
                        child.degree()
                    )));
                }

                builder = builder
                    .with_target(MarkedStatement::from(Arc::new(
                        StatementParser::new(child.front().unwrap())
                            .with_params(&mut params)
                            .with_variables()
                            .parse()
                            .map_err(SemanticError::Other)?,
                    )))
                    .map_err(|e| SemanticError::Other(e.to_string()))?;
            } else {
                builder = builder.with_condition(MarkedStatement::from(Arc::new(
                    StatementParser::new(child)
                        .with_params(&mut params)
                        .with_variables()
                        .parse()
                        .map_err(SemanticError::Other)?,
                )));
            }
        }
        builder
            .build()
            .map_err(|e| SemanticError::Other(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser::LangParser, predefine::setup, statement::term::Term};
    use trees::tr;

    #[test]
    fn problem_parse_test() {
        setup();
        let test = r#"problem {
                        2*x+5 == 0;
                        target find x;
                    };"#;

        let states = LangParser::new().parse(test).unwrap();
        assert_eq!(states.len(), 1);

        let result = ProblemParser::with(&states[0]).parse();
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.conditions.len(), 1);
        assert_eq!(
            *problem.conditions[0].statement,
            (tr(Term::with_symbol_name("==").unwrap()) /
                (tr(Term::with_symbol_name("+").unwrap()) /
                    (tr(Term::with_symbol_name("*").unwrap()) /
                        tr(Term::Number(2.into())) /
                        tr(Term::Variable(1))) /
                    tr(Term::Number(5.into()))) /
                tr(Term::Number(0.into())))
            .into()
        );
    }
}

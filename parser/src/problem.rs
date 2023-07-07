use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

use mcore::{
    problem::{Problem, ProblemBuilder},
    statement::MarkedStatement,
    NormalizationLevel,
};

use crate::ParserError;

use super::{statement::StatementParser, Tree};

pub struct ProblemParser<'a> {
    syntax_tree: &'a Tree,
}

impl<'a> ProblemParser<'a> {
    pub fn with(syntax_tree: &'a Tree) -> Self {
        Self { syntax_tree }
    }

    pub fn parse(self) -> Result<Problem, ParserError> {
        if self.syntax_tree.root().data().symbol != "Problem" {
            return Err(ParserError {
                loc: self.syntax_tree.root().data().location.clone(),
                msg: "expected 'problem'".to_owned(),
            });
        }
        let mut hasher = DefaultHasher::new();
        self.syntax_tree.root().hash(&mut hasher);
        let hash = hasher.finish();

        let mut builder = ProblemBuilder::default().with_id(hash);

        for child in self.syntax_tree.iter() {
            if child.data().symbol == "Target" {
                if child.degree() != 1 {
                    return Err(ParserError {
                        loc: child.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }

                builder = builder
                    .with_target(MarkedStatement::from(Arc::new(
                        StatementParser::new(child.front().unwrap())
                            .with_variables()
                            .parse()?,
                    )))
                    .map_err(|e| ParserError {
                        loc: child.data().location.clone(),
                        msg: e.to_string(),
                    })?;
            } else {
                builder = builder.with_condition(MarkedStatement::from(Arc::new(
                    StatementParser::new(child)
                        .with_variables()
                        .parse()?
                        .normalize(NormalizationLevel::max()),
                )));
            }
        }
        builder.build().map_err(|e| ParserError {
            loc: self.syntax_tree.data().location.clone(),

            msg: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use trees::tr;

    use mcore::statement::term::Term;

    use crate::lang;

    use super::*;

    #[test]
    fn problem_parse_test() {
        let test = r#"problem {
                        target find(x);
                        2*x+5 == 0;
                    }"#;

        let states = lang::problem(test).unwrap();
        let result = ProblemParser::with(&states).parse();
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.conditions.len(), 1);
        assert_eq!(
            *problem.conditions[0].statement,
            (tr(Term::with_symbol_name("==").unwrap()) /
                (tr(Term::with_symbol_name("+").unwrap()) /
                    (tr(Term::with_symbol_name("*").unwrap()) /
                        tr(Term::Number(2.into())) /
                        tr(Term::Variable("x".parse().unwrap()))) /
                    tr(Term::Number(5.into()))) /
                tr(Term::Number(0.into())))
            .into()
        );
    }
}

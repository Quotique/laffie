use super::{statement::StatementParser, SemanticError, Tree};
use crate::{
    problem::{Problem, ProblemBuilder},
    statement::{MarkedStatement, ParamsMap, Statement},
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
        if self.syntax_tree.root().data != "Problem" {
            return Err(SemanticError::UnexpectedWord("Problem".into()));
        }
        let mut builder = ProblemBuilder::new();

        let mut hasher = DefaultHasher::new();
        self.syntax_tree.root().hash(&mut hasher);
        let hash = hasher.finish();
        let mut params = ParamsMap::new();

        for child in self.syntax_tree.iter() {
            if child.data == "Target" {
                if child.degree() != 1 {
                    return Err(SemanticError::WorngArgCount(format!(
                        "target: expect 1 found {}",
                        child.degree()
                    )));
                }

                builder = builder
                    .with_target(MarkedStatement::from(Arc::new(
                        StatementParser::new(child.first().unwrap())
                            .with_params(&mut params)
                            .parse()
                            .map_err(|e| SemanticError::Other(e))?,
                    )))
                    .map_err(|e| SemanticError::Other(e.to_string()))?;
            } else {
                builder = builder.with_condition(MarkedStatement::from(Arc::new(
                    StatementParser::new(child)
                        .with_params(&mut params)
                        .parse()
                        .map_err(|e| SemanticError::Other(e))?,
                )));
            }
        }
        Ok(builder
            .build()
            .map_err(|e| SemanticError::Other(e.to_string()))?)
    }
}

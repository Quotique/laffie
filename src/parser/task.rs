use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use solver::{
    task::{Task, TaskBuilder, TermProps},
    NormalizationLevel,
};

use crate::ParserError;

use super::{term::TermParser, Tree};

pub struct TaskParser<'a> {
    syntax_tree: &'a Tree,
}

impl<'a> TaskParser<'a> {
    pub fn with(syntax_tree: &'a Tree) -> Self {
        Self { syntax_tree }
    }

    pub fn parse(self) -> Result<Task, ParserError> {
        if self.syntax_tree.root().data().symbol != "Task" {
            return Err(ParserError {
                loc: self.syntax_tree.root().data().location.clone(),
                msg: "expected 'task'".to_owned(),
            });
        }
        let mut hasher = DefaultHasher::new();
        self.syntax_tree.root().hash(&mut hasher);
        let hash = hasher.finish();

        let mut builder = TaskBuilder::default().with_id(hash);

        for child in self.syntax_tree.iter() {
            match child.data().symbol.as_str() {
                "Goal" => {
                    builder = builder
                        .with_goal(TermProps::from(
                            TermParser::default().with_variables().try_parse(
                                child.front().ok_or_else(|| ParserError {
                                    loc: child.data().location.clone(),
                                    msg: "must have one argument".to_owned(),
                                })?,
                            )?,
                        ))
                        .map_err(|e| ParserError {
                            loc: child.data().location.clone(),
                            msg: e.to_string(),
                        })?
                }
                "Text" => {
                    builder = builder.with_text(
                        child
                            .front()
                            .ok_or_else(|| ParserError {
                                loc: child.data().location.clone(),
                                msg: "must have one argument".to_owned(),
                            })?
                            .data()
                            .symbol
                            .to_string(),
                    );
                }
                "Answer" => {
                    for i in child.iter() {
                        builder = builder.with_answer(
                            TermParser::default()
                                .with_variables()
                                .try_parse(i)?
                                .normalize(NormalizationLevel::max()),
                        )
                    }
                }
                _ => {
                    builder = builder.with_condition(TermProps::from(
                        TermParser::default()
                            .with_variables()
                            .try_parse(child)?
                            .normalize(NormalizationLevel::max()),
                    ));
                }
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
    use solver::term::Term;

    use crate::lang;

    use super::*;

    #[test]
    fn task_parse_test() {
        let test = r#"task {
                        goal find(x);
                        2*x+5 == 0;
                    }"#;

        let states = lang::task(test).unwrap();
        let result = TaskParser::with(&states).parse();
        assert!(result.is_ok());

        let task = result.unwrap();
        assert_eq!(task.givens.len(), 1);
        println!("{:?}", task.givens[0].term);
        assert_eq!(
            *task.givens[0].term,
            Term::symbol("==")
                .with_child(
                    Term::symbol("+")
                        .with_child(
                            Term::symbol("*")
                                .with_child(Term::number(2))
                                .with_child(Term::variable("x"))
                        )
                        .with_child(Term::number(5))
                )
                .with_child(Term::number(0))
        );
    }
}

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    rc::Rc,
};

use solver::{
    task::{Task, TaskBuilder},
    term::TermProps,
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
                "Purpose" => {
                    builder = builder
                        .with_purpose(TermProps::from(Rc::new(
                            TermParser::new(child.front().ok_or_else(|| ParserError {
                                loc: child.data().location.clone(),
                                msg: "must have one argument".to_owned(),
                            })?)
                            .with_variables()
                            .parse()?,
                        )))
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
                            TermParser::new(i)
                                .with_variables()
                                .parse()?
                                .normalize(NormalizationLevel::max()),
                        )
                    }
                }
                _ => {
                    builder = builder.with_condition(TermProps::from(Rc::new(
                        TermParser::new(child)
                            .with_variables()
                            .parse()?
                            .normalize(NormalizationLevel::max()),
                    )));
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
    use trees::tr;

    use solver::symbol::Symbol;

    use crate::lang;

    use super::*;

    #[test]
    fn task_parse_test() {
        let test = r#"task {
                        purpose find(x);
                        2*x+5 == 0;
                    }"#;

        let states = lang::task(test).unwrap();
        let result = TaskParser::with(&states).parse();
        assert!(result.is_ok());

        let task = result.unwrap();
        assert_eq!(task.conditions.len(), 1);
        assert_eq!(
            *task.conditions[0].term,
            (tr(Symbol::with_func_symbol("==")) /
                (tr(Symbol::with_func_symbol("+")) /
                    (tr(Symbol::with_func_symbol("*")) /
                        tr(Symbol::with_number(2)) /
                        tr(Symbol::Variable("x".parse().unwrap()))) /
                    tr(Symbol::Number(5.into()))) /
                tr(Symbol::Number(0.into())))
            .into()
        );
    }
}

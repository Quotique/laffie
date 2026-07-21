use solver::{
    NormLevel,
    task::{Task, TaskBuilder, TermProps, content_id},
};

use crate::ParserError;

use super::{Tree, term::TermParser};

pub struct TaskParser<'a> {
    syntax_tree: &'a Tree,
}

impl<'a> From<&'a Tree> for TaskParser<'a> {
    fn from(syntax_tree: &'a Tree) -> Self {
        Self { syntax_tree }
    }
}

impl<'a> TaskParser<'a> {
    pub fn parse(self) -> Result<Task, ParserError> {
        if self.syntax_tree.root().data().symbol != "Task" {
            return Err(ParserError {
                loc: self.syntax_tree.root().data().location.clone(),
                msg: "expected 'task'".to_owned(),
            });
        }
        let mut builder = TaskBuilder::default();

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
                "Id" => {
                    builder = builder.with_name(
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
                                .normalize(NormLevel::Full),
                        )
                    }
                }
                _ => {
                    builder = builder.with_condition(TermProps::from(
                        TermParser::default()
                            .with_variables()
                            .try_parse(child)?
                            .normalize(NormLevel::Full),
                    ));
                }
            }
        }
        let mut task = builder.build().map_err(|e| ParserError {
            loc: self.syntax_tree.data().location.clone(),
            msg: e.to_string(),
        })?;
        // Content id from the parsed terms, not the syntax tree (location-free).
        task.id = content_id(&task.givens, &task.goal);
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use solver::term::{TermBuf, var};

    use crate::lang;

    use super::*;

    #[test]
    fn task_parse_test() {
        let test = r#"task {
                        goal find(x);
                        2*x+5 == 0;
                    }"#;

        let states = lang::task(test).unwrap();
        let result = TaskParser::from(&states).parse();
        assert!(result.is_ok());

        let task = result.unwrap();
        assert!(task.name.is_empty());
        assert_eq!(task.givens.len(), 1);
        println!("{:?}", task.givens[0].term);
        assert_eq!(
            *task.givens[0].term,
            TermBuf::symbol("==")
                .arg(
                    TermBuf::symbol("+")
                        .arg(TermBuf::symbol("*").arg(TermBuf::number(2)).arg(var("x")))
                        .arg(TermBuf::number(5))
                )
                .arg(TermBuf::zero())
        );
    }

    #[test]
    fn task_id_parse_test() {
        let test = r#"task {
                        goal find(x);
                        id "c26";
                        text "biquadratic";
                        2*x+5 == 0;
                    }"#;

        let states = lang::task(test).unwrap();
        let task = TaskParser::from(&states).parse().unwrap();
        assert_eq!(task.name, "c26");
        assert_eq!(task.text, "biquadratic");
        assert_eq!(task.givens.len(), 1);
    }
}

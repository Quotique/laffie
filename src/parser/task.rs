use solver::{
    NormLevel,
    task::{Goal, Task, TaskBuilder, content_id},
    term::SharedTerm,
};

use crate::{
    Names, ParserError,
    scope::TermScope,
    source::Source,
    syntax::{self, Expr},
};

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
        // The grammar admits exactly one `goal` statement per task.
        let goal_node = self
            .syntax_tree
            .iter()
            .find(|child| child.data().symbol.as_str() == "Goal")
            .ok_or_else(|| ParserError {
                loc: self.syntax_tree.data().location.clone(),
                msg: "task has no goal".to_owned(),
            })?;
        let goal =
            TermParser::default()
                .with_variables()
                .try_parse(goal_node.front().ok_or_else(|| ParserError {
                    loc: goal_node.data().location.clone(),
                    msg: "must have one argument".to_owned(),
                })?)?;
        let goal = Goal::parse(goal).map_err(|e| ParserError {
            loc: goal_node.data().location.clone(),
            msg: e.to_string(),
        })?;
        let mut builder = TaskBuilder::from_goal(goal);

        for child in self.syntax_tree.iter() {
            match child.data().symbol.as_str() {
                "Goal" => {}
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
                    builder = builder.with_condition(SharedTerm::new(
                        TermParser::default()
                            .with_variables()
                            .try_parse(child)?
                            .normalize(NormLevel::Full),
                    ));
                }
            }
        }
        let mut task = builder.build();
        // Content id from the parsed terms, not the syntax tree (location-free).
        task.id = content_id(&task.givens, task.goal());
        Ok(task)
    }
}

impl syntax::Task {
    pub(crate) fn build(&self, src: &Source) -> Result<Task, ParserError> {
        let term = |e: &Expr| TermScope::new(src, Names::Variables).term(e);
        let goal = Goal::parse(term(&self.goal)?).map_err(|e| src.error(self.goal.span, e))?;
        let mut builder = TaskBuilder::from_goal(goal).with_text(self.text.clone());
        if let Some(name) = &self.name {
            builder = builder.with_name(name.clone());
        }
        for answer in &self.expected {
            builder = builder.with_answer(term(answer)?.normalize(NormLevel::Full));
        }
        for given in &self.givens {
            builder =
                builder.with_condition(SharedTerm::new(term(given)?.normalize(NormLevel::Full)));
        }
        let mut task = builder.build();
        // The id hashes terms only, so moving a task keeps it.
        task.id = content_id(&task.givens, task.goal());
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
        println!("{:?}", task.givens[0]);
        assert_eq!(
            *task.givens[0],
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

    #[test]
    fn a_capitalized_keyword_is_a_plain_condition() {
        let task = crate::task("task { goal find(x); Goal; Answer; Text; Id; }").unwrap();
        assert_eq!(task.givens.len(), 4);
    }
}

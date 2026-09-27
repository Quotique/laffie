use solver::{
    NormLevel,
    task::{Goal, Task, TaskBuilder, content_id},
    term::SharedTerm,
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

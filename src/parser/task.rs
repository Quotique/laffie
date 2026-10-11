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

impl syntax::Task {
    pub(crate) fn build(&self, src: &Source) -> Result<Task, ParserError> {
        let term = |e: &Expr| TermScope::new(src, Names::Variables).term(e);
        let goal = Goal::parse(term(&self.goal)?).map_err(|e| src.error(self.goal.span, e))?;
        let mut builder = TaskBuilder::from_goal(goal).with_text(self.text.clone());
        if let Some(name) = &self.name {
            builder = builder.with_name(name.clone());
        }
        for answer in &self.answers {
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

    #[test]
    fn a_task_reads_its_conditions_with_variables() {
        let test = r#"task {
                        goal find(x);
                        2*x+5 == 0;
                    }"#;

        let result = crate::task(test);
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
    fn a_task_reads_its_id_and_text() {
        let test = r#"task {
                        goal find(x);
                        id "c26";
                        text "biquadratic";
                        2*x+5 == 0;
                    }"#;

        let task = crate::task(test).unwrap();
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

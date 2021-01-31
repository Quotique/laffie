use crate::statement::MarkedStatement;
use std::fmt;

#[derive(Clone)]
pub struct Problem {
    pub id:               u64,
    pub conditions:       Vec<MarkedStatement>,
    pub target:           MarkedStatement,
    pub subproblem_level: usize,
}

#[derive(Clone, Debug)]
pub enum ProblemBuilderError {
    OnlyOneTargetAllowed,
    NoTargetFound,
}

pub struct ProblemBuilder {
    conditions: Vec<MarkedStatement>,
    target:     Option<MarkedStatement>,
}

impl ProblemBuilder {
    pub fn new() -> Self {
        Self {
            conditions: vec![],
            target:     None,
        }
    }

    pub fn with_target(mut self, mut target: MarkedStatement) -> Result<Self, ProblemBuilderError> {
        if let Some(x) = self.target.replace(target) {
            Err(ProblemBuilderError::OnlyOneTargetAllowed)
        } else {
            Ok(self)
        }
    }

    pub fn with_condition(mut self, mut condition: MarkedStatement) -> Self {
        condition.weight = 0;
        self.conditions.push(condition);
        self
    }

    pub fn with_conditions<I>(mut self, reqs: I) -> Self
    where
        I: std::iter::Iterator<Item = MarkedStatement> + IntoIterator<Item = MarkedStatement>,
    {
        self.conditions.extend(reqs.map(|mut x| {
            x.weight = 0;
            x
        }));
        self
    }

    pub fn build(self) -> Result<Problem, ProblemBuilderError> {
        Ok(Problem {
            id:               0,
            conditions:       self.conditions,
            target:           self.target.ok_or(ProblemBuilderError::NoTargetFound)?,
            subproblem_level: 0,
        })
    }
}

impl Problem {
    // fn new(target: ProblemType) -> Problem {
    //     Problem {
    //         id: 1,
    //         target,
    //         conditions: vec![],
    //         subproblem_level: 0,
    //     }
    // }

    // pub fn from(node: &ParserNode) -> Result<Problem, String> {
    //     if node.data != "Problem" {
    //         return Err(format!("Bad root node: {:?}", node));
    //     }
    //
    //     let mut result = Problem::new(ProblemType::Transform);
    //     let mut s = DefaultHasher::new();
    //     node.hash(&mut s);
    //     result.id = s.finish();
    //     let mut params = HashMap::new();
    //
    //     for child in node.iter() {
    //         if child.data == "Target" {
    //             result.target = ProblemType::try_from(child, &mut params)?;
    //         } else {
    //             result
    //                 .conditions
    //                 .push(MarkedStatement::from(Arc::new(Statement::new(
    //                     child,
    //                     &mut params,
    //                 )?)));
    //         }
    //     }
    //     Ok(result)
    // }
}

impl fmt::Display for ProblemBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OnlyOneTargetAllowed => write!(f, "Duplicate target"),
            Self::NoTargetFound => write!(f, "No target found"),
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{id: {:x}, contitions: [{}], target: {}, subproblem_level: {} }}",
            self.id,
            self.conditions
                .iter()
                .map(|x| x.statement.to_string())
                .collect::<Vec<String>>()
                .join(";"),
            0, // self.target,
            self.subproblem_level,
        )
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use bigdecimal::BigDecimal as Decimal;
    use core::{symbols::symbols_tests::setup, term::Term};
    use std::str::FromStr;
    use trees::linked::fully::tr;

    // pub fn test_problem() -> Problem {
    //     let test = tr(String::from("Problem")) /
    //         (tr(String::from("==")) /
    //             (tr(String::from("+")) /
    //                 (tr(String::from("*")) / tr(String::from("2")) /
    // tr(String::from("x"))) /                 tr(String::from("5"))) /
    //             tr(String::from("0"))) /
    //         (tr(String::from("Target")) / (tr(String::from("find")) /
    // tr(String::from("x"))));
    //
    //     Problem::from(test).unwrap()
    // }

    #[test]
    fn problem_parse_test() {
        // setup();
        //
        // let problem = test_problem();
        // assert_eq!(problem.conditions.len(), 1);
        // assert_eq!(
        //     problem.conditions[0].statement.root,
        //     tr(Term::Symbol(1)) /
        //         (tr(Term::Symbol(2)) /
        //             (tr(Term::Symbol(7)) /
        //                 tr(Term::Number(Decimal::from_str("2").unwrap())) /
        //                 tr(Term::Variable(1))) /
        //             tr(Term::Number(Decimal::from_str("5").unwrap()))) /
        //         tr(Term::Number(Decimal::from_str("0").unwrap()))
        // );
    }
}

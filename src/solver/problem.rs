use std::{collections::HashMap, fmt, io, path::Path, sync::Arc};

use core::{
    dir_parser::load_dir,
    statement::{ParamsMap, Statement},
    term::StatementTree,
};

use trees::Node;

type ParserNode = Node<String>;

#[derive(Debug, Clone)]
pub enum ProblemType {
    Proof(Arc<Statement>),
    Calculate(StatementTree),
    Transform,
}

pub struct Problem {
    pub conditions: Vec<Arc<Statement>>,
    pub target:     ProblemType,
}

pub struct ProblemStorage {
    pub problems: Vec<Problem>,
}

impl ProblemType {
    fn try_from(node: &ParserNode, params: &mut ParamsMap) -> Result<Self, String> {
        if node.degree() != 2 {
            return Err("Wrong target tree".into());
        }
        let target = Statement::new(node.last().unwrap(), params)?;

        match node.first().unwrap().data.as_ref() {
            "proof" => Ok(ProblemType::Proof(Arc::new(target))),
            "find" => Ok(ProblemType::Calculate(target.root)),
            "transform" => Ok(ProblemType::Transform),
            _ => Err(format!("Incorrect problem type: {}", node.first().unwrap().data)),
        }
    }
}

impl fmt::Display for ProblemType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProblemType::Proof(target) => write!(f, "proof {}", target),
            ProblemType::Calculate(target) => write!(f, "find {}", target),
            ProblemType::Transform => write!(f, "transform"),
        }
    }
}

impl Problem {
    fn new(target: ProblemType) -> Problem {
        Problem {
            target,
            conditions: vec![],
        }
    }

    pub fn from(node: &ParserNode) -> Result<Problem, String> {
        if node.data != "Problem" {
            return Err(format!("Bad root node: {:?}", node));
        }

        let mut result = Problem::new(ProblemType::Transform);
        let mut params = HashMap::new();

        for child in node.iter() {
            if child.data == "Target" {
                result.target = ProblemType::try_from(child, &mut params)?;
            } else {
                result.conditions.push(Arc::new(Statement::new(child, &mut params)?));
            }
        }
        Ok(result)
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{contitions: [{}], target: {} }}",
            self.conditions
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(";"),
            self.target,
        )
    }
}

impl ProblemStorage {
    pub fn new() -> ProblemStorage {
        ProblemStorage { problems: Vec::new() }
    }

    pub fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        load_dir(dir, &mut |s| {
            trace!("New problem cb: {}", s);
            if s.root().data == "Problem" {
                match Problem::from(&s) {
                    Ok(p) => self.problems.push(p),
                    Err(e) => error!("Problem not parsed: {}", e),
                }
            }
        })
    }
}

#[cfg(test)]
pub mod problem_tests {
    use super::*;
    use bigdecimal::BigDecimal as Decimal;
    use core::{symbols::symbols_tests::setup, term::Term};
    use solver::trees::linked::fully::tr;
    use std::str::FromStr;

    pub fn test_problem() -> Problem {
        let test = tr(String::from("Problem")) /
            (tr(String::from("==")) /
                (tr(String::from("+")) /
                    (tr(String::from("*")) / tr(String::from("2")) / tr(String::from("x"))) /
                    tr(String::from("5"))) /
                tr(String::from("0"))) /
            (tr(String::from("Target")) / tr(String::from("find")) / tr(String::from("x")));

        Problem::from(&test).unwrap()
    }

    #[test]
    fn problem_parse_test() {
        setup();

        let problem = test_problem();
        assert_eq!(problem.conditions.len(), 1);
        assert_eq!(
            problem.conditions[0].root,
            tr(Term::Symbol(1)) /
                (tr(Term::Symbol(2)) /
                    (tr(Term::Symbol(7)) /
                        tr(Term::Number(Decimal::from_str("2").unwrap())) /
                        tr(Term::Variable(1))) /
                    tr(Term::Number(Decimal::from_str("5").unwrap()))) /
                tr(Term::Number(Decimal::from_str("0").unwrap()))
        );
    }
}

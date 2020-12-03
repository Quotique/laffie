use std::{
    cell::RefCell,
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    convert::From,
    fmt,
    hash::{Hash, Hasher},
    io,
    path::Path,
    sync::Arc,
};

use core::{
    dir_parser::load_dir,
    statement::{ParamsMap, Statement},
    term::StatementTree,
};

use trees::Node;

type ParserNode = Node<String>;

pub const DEFAULT_WEIGHT: usize = 10;

#[derive(Debug, Clone)]
pub struct MarkedStatement {
    pub statement:     Arc<Statement>,
    pub applied_rules: RefCell<HashSet<usize>>,
    pub weight:        RefCell<usize>,
    pub replaced:      RefCell<bool>,
}

#[derive(Debug, Clone)]
pub enum ProblemType {
    Proof(MarkedStatement),
    Calculate(StatementTree),
    Transform,
}

pub struct Problem {
    pub id:               u64,
    pub conditions:       Vec<MarkedStatement>,
    pub target:           ProblemType,
    pub subproblem_level: usize,
}

pub struct ProblemStorage {
    pub problems: Vec<Problem>,
}

impl From<Arc<Statement>> for MarkedStatement {
    fn from(statement: Arc<Statement>) -> Self {
        Self {
            statement:     statement,
            applied_rules: RefCell::new(HashSet::new()),
            weight:        RefCell::new(DEFAULT_WEIGHT),
            replaced:      RefCell::new(false),
        }
    }
}

impl MarkedStatement {
    pub fn normalize(self) -> Self {
        let mut new_statement = (*self.statement).clone();
        super::operations::normalize(new_statement.root.root_mut());
        MarkedStatement {
            statement:     Arc::new(new_statement),
            applied_rules: self.applied_rules,
            weight:        self.weight,
            replaced:      self.replaced,
        }
    }
}

impl ProblemType {
    pub fn try_from(node: &ParserNode, params: &mut ParamsMap) -> Result<Self, String> {
        if node.degree() != 1 {
            return Err("Wrong target tree".into());
        }
        let label = node.first().unwrap();

        if label.degree() != 1 {
            return Err("Wrong target tree".into());
        }
        let target = Statement::new(label.first().unwrap(), params)?;

        match label.data.as_ref() {
            "proof" => Ok(ProblemType::Proof(MarkedStatement::from(Arc::new(target)))),
            "find" => Ok(ProblemType::Calculate(target.root)),
            "transform" => Ok(ProblemType::Transform),
            _ => Err(format!("Incorrect problem type: {}", label.data)),
        }
    }
}

impl fmt::Display for ProblemType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProblemType::Proof(target) => write!(f, "proof {}", target.statement),
            ProblemType::Calculate(target) => write!(f, "find {}", target),
            ProblemType::Transform => write!(f, "transform"),
        }
    }
}

impl Problem {
    fn new(target: ProblemType) -> Problem {
        Problem {
            id: 1,
            target,
            conditions: vec![],
            subproblem_level: 0,
        }
    }

    pub fn from(node: &ParserNode) -> Result<Problem, String> {
        if node.data != "Problem" {
            return Err(format!("Bad root node: {:?}", node));
        }

        let mut result = Problem::new(ProblemType::Transform);
        let mut s = DefaultHasher::new();
        node.hash(&mut s);
        result.id = s.finish();
        let mut params = HashMap::new();

        for child in node.iter() {
            if child.data == "Target" {
                result.target = ProblemType::try_from(child, &mut params)?;
            } else {
                result
                    .conditions
                    .push(MarkedStatement::from(Arc::new(Statement::new(
                        child,
                        &mut params,
                    )?)));
            }
        }
        Ok(result)
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
            self.target,
            self.subproblem_level,
        )
    }
}

impl ProblemStorage {
    pub fn new() -> ProblemStorage {
        ProblemStorage {
            problems: Vec::new(),
        }
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
            (tr(String::from("Target")) / (tr(String::from("find")) / tr(String::from("x"))));

        Problem::from(&test).unwrap()
    }

    #[test]
    fn problem_parse_test() {
        setup();

        let problem = test_problem();
        assert_eq!(problem.conditions.len(), 1);
        assert_eq!(
            problem.conditions[0].statement.root,
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

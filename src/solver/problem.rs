use std::collections::HashMap;
use std::fmt;

use parser::syntax_tree::Node as ParserNode;

use core::node::Node;
use core::statement::{ParamsMap, Statement};

extern crate log;

pub enum ProblemType {
    Proof,
    Calculate,
    Transform,
}

pub struct Problem {
    pub problem_type: ProblemType,
    pub conditions: Vec<Statement>,
    pub targets: Option<Node>,
}

pub struct Solution {
    pub targets: Vec<Node>,
}

impl ProblemType {
    fn from(s: &str) -> Option<ProblemType> {
        match s {
            "proof" => Some(ProblemType::Proof),
            "find" => Some(ProblemType::Calculate),
            "transform" => Some(ProblemType::Transform),
            _ => None,
        }
    }
}

impl fmt::Display for ProblemType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProblemType::Proof => write!(f, "proof"),
            ProblemType::Calculate => write!(f, "find"),
            ProblemType::Transform => write!(f, "transform"),
        }
    }
}

impl Problem {
    fn new(problem_type: ProblemType) -> Problem {
        Problem {
            problem_type,
            conditions: vec![],
            targets: None,
        }
    }

    pub fn from(node: &ParserNode) -> Option<Problem> {
        if node.label != "Problem" {
            error!(target: "problem", "Bad root node: {:?}", node);
            return None;
        }

        let mut result = Problem::new(ProblemType::Calculate);
        let mut params = HashMap::new();

        for child in node.childs.iter() {
            if child.label == "Target" {
                result.parse_target(child, &mut params);
            } else {
                match Statement::new(child, &mut params) {
                    Some(s) => result.conditions.push(s),
                    None => return None,
                }
            }
        }
        Some(result)
    }

    fn parse_target(&mut self, node: &ParserNode, params: &mut ParamsMap) {
        if node.childs.len() != 2 {
            return;
        }

        match ProblemType::from(&node.childs[0].label) {
            Some(t) => self.problem_type = t,
            None => {
                error!("Incorrect problem type: {}", node.childs[0].label);
                return;
            }
        }
        match Statement::new(&node.childs[1], params) {
            Some(s) => self.targets = Some(s.root),
            None => {
                error!("Bad target body");
                return;
            }
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{contitions: [{}], target: {} {} }}",
            self.conditions
                .iter()
                .map(|x| Statement::to_string(&x.root))
                .collect::<Vec<String>>()
                .join(";"),
            self.problem_type,
            match &self.targets {
                Some(t) => Statement::to_string(&t),
                None => String::from("None")
            }
        )
    }
}

use std::{collections::HashMap, fmt, io, path::Path};

use core::{dir_parser::load_dir, tree_utils::NodeData};

use super::{
    statement::{ParamsMap, Statement},
    trees::{Node as TreeNode, Tree},
};

use core::rule::RulesEngine;

type ParserTree = Tree<String>;
type ParserNode = TreeNode<String>;
type TargetTree = Tree<NodeData>;

pub enum ProblemType {
    Proof(TargetTree),
    Calculate(TargetTree),
    Transform,
}

pub struct Problem {
    pub conditions: Vec<Statement>,
    pub target:     ProblemType,
}

pub struct Solution {
    pub targets: Vec<TargetTree>,
}

pub struct ProblemStorage {
    pub problems: Vec<Problem>,
}

impl ProblemType {
    fn try_from(node: &ParserNode, params: &mut ParamsMap) -> Result<Self, String> {
        if node.degree() != 2 {
            return Err("Wrong target tree".into());
        }
        let target = Statement::new(node.last().unwrap(), params)?.root;

        match node.first().unwrap().data.as_ref() {
            "proof" => Ok(ProblemType::Proof(target)),
            "find" => Ok(ProblemType::Calculate(target)),
            "transform" => Ok(ProblemType::Transform),
            _ => Err(format!("Incorrect problem type: {}", node.first().unwrap().data)),
        }
    }
}

impl fmt::Display for ProblemType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProblemType::Proof(target) => write!(f, "proof {:?}", target),
            ProblemType::Calculate(target) => write!(f, "find {:?}", target),
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
                result.conditions.push(Statement::new(child, &mut params)?);
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
            if s.root().data == "Problem" {
                match Problem::from(&s) {
                    Ok(p) => self.problems.push(p),
                    Err(e) => error!("Problem not parsed: {}", e),
                }
            }
        })
    }
}

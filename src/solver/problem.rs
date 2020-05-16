use std::{collections::HashMap, fmt, io, path::Path, sync::Arc};

use core::{dir_parser::load_dir, symbols::symbol_by_name, tree_utils::NodeData};

use super::{
    statement::{ParamsMap, Statement},
    trees::{Node as TreeNode, Tree},
};

use bigdecimal::BigDecimal;
use core::rule::RulesEngine;
use std::collections::HashSet;

// type ParserTree = Tree<String>;
type ParserNode = TreeNode<String>;
type TargetTree = Tree<NodeData>;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub conditions: Vec<Arc<Statement>>,
    pub target:     ProblemType,
    pub answer:     Option<Arc<Statement>>,
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

impl Solution {
    pub fn new(problem: &Problem) -> Solution {
        Solution {
            target:     problem.target.clone(),
            conditions: problem.conditions.iter().map(|x| Arc::new(x.clone())).collect(),
            answer:     None,
        }
    }

    pub fn solve(&mut self, rules_engine: &RulesEngine) -> Result<(), String> {
        loop {
            let mut new_state = None;
            match self.conditions.iter().max_by(|x, y| x.weigth().cmp(&y.weigth())) {
                Some(state) => {
                    trace!("Statement: {} ({})", state, state.weigth());
                    if !state.decrease_weigth() {
                        return Err("No solution found".into());
                    }
                    if self.is_answer(&state) {
                        self.answer = Some(state.clone());
                        return Ok(());
                    }

                    let mut rules = rules_engine.find_rules(&state.symbols);
                    rules.sort_by(|x, y| {
                        x.read()
                            .expect("Cant lock rule")
                            .level
                            .cmp(&y.read().expect("Cant lock rule").level)
                    });

                    for rule in rules.iter() {
                        trace!("Rule: {:?}", rule);
                        match Statement::apply(state.clone(), rule.clone()) {
                            Ok(s) => {
                                new_state = Some(s);
                                break;
                            }
                            Err(e) => {
                                trace!("Cant apply rule: {}", e);
                            }
                        }
                    }
                }
                None => return Err("Conditions not found".into()),
            }
            if let Some(s) = new_state {
                self.conditions.push(Arc::new(s));
            }
        }
    }

    pub fn is_answer(&self, statement: &Statement) -> bool {
        match &self.target {
            ProblemType::Calculate(x) => {
                let eq_sym = symbol_by_name(&String::from("==")).unwrap().id;
                if statement.root.degree() != 2 || statement.root.root().data != NodeData::Symbol(eq_sym) {
                    return false;
                }
                statement.root.first().unwrap() == x.root()
            }
            ProblemType::Proof(x) => self.is_true(x),
            _ => false,
        }
    }

    fn is_true(&self, statement: &TargetTree) -> bool {
        match statement.data {
            NodeData::Symbol(id) => {
                if id == symbol_by_name(&">".into()).unwrap().id {
                    if let (NodeData::Number(d1), NodeData::Number(d2)) =
                        (&statement.first().unwrap().data, &statement.last().unwrap().data)
                    {
                        return d1 > d2;
                    }
                }
                false
            }
            _ => false,
        }
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

impl fmt::Display for Solution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(a) = self.answer.as_ref() {
            let mut trace: Vec<Arc<Statement>> = vec![];

            fn visitor(trace: &mut Vec<Arc<Statement>>, state: &Arc<Statement>) {
                trace.push(state.clone());
                for i in state.parents.iter() {
                    visitor(trace, i);
                }
            };

            visitor(&mut trace, a);

            while let Some(t) = trace.pop() {
                write!(f, "\n")?;
                for p in t.parents.iter() {
                    write!(f, "{},\n", p)?;
                }
                write!(f, "------------------------------------------------------------\n")?;
                write!(f, "{}\n", t)?;
            }
            write!(f, "SOLVED!\n")?;
        } else {
            write!(f, "NOT SOLVED\n")?;
        }

        write!(
            f,
            "######################################################################\n"
        )
    }
}

#[cfg(test)]
mod paroblem_tests {
    use super::*;
    use core::symbols::symbols_tests::setup;
    use solver::trees::linked::fully::tr;

    fn test_problem() -> Problem {
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
            tr(NodeData::Symbol(1)) /
                (tr(NodeData::Symbol(2)) /
                    (tr(NodeData::Symbol(7)) / tr(NodeData::Symbol(6)) / tr(NodeData::Param(1))) /
                    tr(NodeData::Symbol(5))) /
                tr(NodeData::Symbol(4))
        );
    }

    #[test]
    fn check_answer_test() {
        setup();

        let problem = test_problem();
        let solution = Solution::new(&problem);
        let statement_answer =
            Statement::from(tr(NodeData::Symbol(1)) / tr(NodeData::Param(1)) / tr(NodeData::Symbol(5)));
        let statement_not_answer =
            Statement::from(tr(NodeData::Symbol(1)) / tr(NodeData::Param(2)) / tr(NodeData::Symbol(5)));
        assert_eq!(solution.is_answer(&statement_answer), true);
        assert_eq!(solution.is_answer(&statement_not_answer), false);
    }
}

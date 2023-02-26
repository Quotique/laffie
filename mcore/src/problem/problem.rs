use crate::statement::MarkedStatement;
use std::{fmt, iter::Iterator};

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

#[derive(Default)]
pub struct ProblemBuilder {
    id:         u64,
    conditions: Vec<MarkedStatement>,
    target:     Option<MarkedStatement>,

    subproblem_level: usize,
}

impl ProblemBuilder {
    pub fn with_id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }

    pub fn with_target(mut self, target: MarkedStatement) -> Result<Self, ProblemBuilderError> {
        if let Some(_x) = self.target.replace(target) {
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

    pub fn with_conditions(mut self, reqs: impl Iterator<Item = MarkedStatement>) -> Self {
        self.conditions.extend(reqs.map(|mut x| {
            x.weight = 0;
            x
        }));
        self
    }

    pub fn with_level(mut self, level: usize) -> Self {
        self.subproblem_level = level;
        self
    }

    pub fn build(self) -> Result<Problem, ProblemBuilderError> {
        Ok(Problem {
            id:               self.id,
            conditions:       self.conditions,
            target:           self.target.ok_or(ProblemBuilderError::NoTargetFound)?,
            subproblem_level: self.subproblem_level,
        })
    }
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
            "{:x} {}\n  {}",
            self.id,
            self.target,
            self.conditions
                .iter()
                .map(|x| x.statement.to_string())
                .collect::<Vec<String>>()
                .join("\n  "),
        )
    }
}

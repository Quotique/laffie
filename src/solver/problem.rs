use core::Node;
use core::Statement;

enum ProblemType {
    Proof,
    Calculate,
    Transform,
}

pub struct Problem {
    pub problem_type: ProblemType,
    pub conditions: Vec<Statement>,
    pub targets: Vec<Node>,
}

pub struct Solution {
    pub targets: Vec<Node>,
}

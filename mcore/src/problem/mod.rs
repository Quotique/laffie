mod frame;
mod problem;
mod solution;
mod target;

pub use self::{
    problem::{Problem, ProblemBuilder},
    solution::Solution,
};

#[cfg(test)]
pub fn parse_problem(text: &'static str) -> Problem {
    let states = parser::lang::problem(text).unwrap();
    let problem = parser::ProblemParser::with(&states).parse().unwrap();

    unsafe { std::mem::transmute::<_, Problem>(problem) }
}

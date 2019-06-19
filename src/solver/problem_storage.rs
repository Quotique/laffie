use std::path::Path;
use std::{fs, io};

use parser::lang;

use super::problem::Problem;
//use Solution;

use core::rules_engine::RulesEngine;

//pub struct SolverEngine<'a> {
//    pub problem: &'a Problem,
//    pub solution: Vec<Statement>,
//    pub result: Solution,
//}
//
//impl<'a> SolverEngine<'a> {
//    pub fn new(problem: &'a Problem) -> SolverEngine<'a> {
//        SolverEngine(problem: problem, solution: vec![], result: Solution())
//    }
//
//    pub fn run(rules: &RulesEngine) -> Option<Solution> {
//    }
//}

pub struct ProblemStorage {
    pub problems: Vec<Problem>,
}

impl ProblemStorage {
    pub fn new() -> ProblemStorage {
        ProblemStorage {
            problems: Vec::new(),
        }
    }

    pub fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        if !dir.is_dir() {
            panic!(dir
                .to_string_lossy()
                .to_string()
                .push_str(" is not directory!"));
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.load_dir(&path)?;
            } else if path.extension().unwrap() == "pbl" {
                self.load_file(&path)?;
            }
        }
        Ok(())
    }

    fn load_file(&mut self, file: &Path) -> io::Result<()> {
        info!("Processing file: {}", file.to_string_lossy());
        let content = fs::read_to_string(file)?;
        let states = lang::StatementsParser::new().parse(&content[..]).unwrap();
        let mut symbol_id: u64 = 0;
        for s in states {
            if s.label == "Problem" {
                match Problem::from(&s) {
                    Some(p) => self.problems.push(p),
                    None => error!("Problem not parsed"),
                }
            }
        }

        Ok(())
    }
}

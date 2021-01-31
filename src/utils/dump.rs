use std::{fs::File, io::prelude::*};

use crate::{problem::Solution, statement::MarkedStatement};

pub trait Dumper {
    fn subproblem_start(&mut self, solution: &Solution);

    fn subproblem_end(&mut self);

    fn add_statement(&mut self, statement: &MarkedStatement);
}

pub struct FileDumper {
    subproblem_level: usize,
    file:             File,
}

impl FileDumper {
    pub fn new(file_name: &str) -> FileDumper {
        FileDumper {
            subproblem_level: 0,
            file:             File::create(file_name).expect("Unable to create dump file"),
        }
    }

    fn prefix(&self) -> String {
        String::from(" ").repeat(self.subproblem_level * 2)
    }
}

impl Dumper for FileDumper {
    fn subproblem_start(&mut self, solution: &Solution) {
        self.file
            .write_all(
                format!(
                    "{} {} [{}]\n",
                    self.prefix(),
                    solution.problem.target,
                    solution
                        .problem
                        .conditions
                        .iter()
                        .map(|x| x.statement.to_string())
                        .collect::<Vec<String>>()
                        .join(";"),
                )
                .as_bytes(),
            )
            .expect("Unable write into dump file");
        self.subproblem_level += 1;
    }

    fn subproblem_end(&mut self) {
        self.subproblem_level -= 1;
    }

    fn add_statement(&mut self, statement: &MarkedStatement) {
        self.file
            .write_all(format!("{} {}\n", self.prefix(), statement.statement).as_bytes())
            .expect("Unable write into dump file");
    }
}

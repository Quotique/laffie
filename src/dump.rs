use std::{fs::File, io::prelude::*};

use crate::solver::{problem::MarkedStatement, solution::Solution};

trait Dumper {
    fn subproblem_start(&mut self, solution: &Solution);

    fn subproblem_end(&mut self);

    fn add_statement(&mut self, statement: &MarkedStatement);
}

pub struct FileDumper {
    subproblem_level: usize,
    file:             File,
}

impl FileDumper {
    pub fn new(file_name: &String) -> FileDumper {
        FileDumper {
            subproblem_level: 0,
            file:             File::create(file_name).expect("Unable to create dump file"),
        }
    }

    fn prefix(&self) -> String {
        String::from(" ").repeat(self.subproblem_level)
    }
}

impl Dumper for FileDumper {
    fn subproblem_start(&mut self, solution: &Solution) {
        self.file
            .write_all(format!("").as_bytes())
            .expect("Unable write into dump file");
        self.subproblem_level += 1;
    }

    fn subproblem_end(&mut self) {
        self.subproblem_level -= 1;
    }

    fn add_statement(&mut self, statement: &MarkedStatement) {}
}

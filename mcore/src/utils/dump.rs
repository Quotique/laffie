use std::{
    fs::File,
    io::prelude::*,
    path::Path,
    sync::{Arc, Mutex},
};

use serde_derive::Deserialize;

use crate::{
    problem::{Problem, SolveStatus},
    statement::MarkedStatement,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub sink:     String,
    pub filename: String,
}

#[derive(Clone)]
pub struct Dumper {
    sink: Arc<Mutex<Box<dyn DumperSink>>>,
}

pub trait DumperSink {
    fn subproblem_start(&mut self, problem: &Problem);

    fn subproblem_end(&mut self, status: &SolveStatus);

    fn add_statement(&mut self, statement: &MarkedStatement, parent: &MarkedStatement);
}

pub struct FileDumper {
    subproblem_level: usize,
    file:             File,
}

pub struct NoneDumper {}

impl Dumper {
    pub fn new(config: Config) -> Self {
        Self {
            sink: Arc::new(Mutex::new(match config.sink.as_str() {
                "file" => Box::new(FileDumper::new(config.filename.as_str())),
                "none" => Box::new(NoneDumper {}),
                _ => Box::new(NoneDumper {}),
            })),
        }
    }
}

impl Default for Dumper {
    fn default() -> Self {
        Self {
            sink: Arc::new(Mutex::new(Box::new(NoneDumper {}))),
        }
    }
}

impl DumperSink for Dumper {
    fn subproblem_start(&mut self, problem: &Problem) {
        self.sink.lock().unwrap().subproblem_start(problem);
    }

    fn subproblem_end(&mut self, status: &SolveStatus) {
        self.sink.lock().unwrap().subproblem_end(status);
    }

    fn add_statement(&mut self, statement: &MarkedStatement, parent: &MarkedStatement) {
        self.sink.lock().unwrap().add_statement(statement, parent);
    }
}

impl DumperSink for NoneDumper {
    fn subproblem_start(&mut self, _: &Problem) {}

    fn subproblem_end(&mut self, _: &SolveStatus) {}

    fn add_statement(&mut self, _: &MarkedStatement, _: &MarkedStatement) {}
}

impl FileDumper {
    pub fn new(file_name: &str) -> FileDumper {
        let path: &Path = file_name.as_ref();
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).expect("unable to create directory");
        }
        FileDumper {
            subproblem_level: 0,
            file:             File::create(path).expect("Unable to create dump file"),
        }
    }

    fn prefix(&self) -> String {
        String::from("┆ ").repeat(self.subproblem_level)
    }
}

impl DumperSink for FileDumper {
    fn subproblem_start(&mut self, problem: &Problem) {
        self.file
            .write_all(
                format!(
                    "{} {} [{}]\n",
                    self.prefix(),
                    problem.target,
                    problem
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

    fn subproblem_end(&mut self, status: &SolveStatus) {
        self.file
            .write_all(
                format!(
                    "{}[{} cycles, {} ms]\n",
                    self.prefix(),
                    status.cycles_count,
                    status.absolute_time
                )
                .as_bytes(),
            )
            .expect("Unable write into dump file");
        self.subproblem_level -= 1;
    }

    fn add_statement(&mut self, statement: &MarkedStatement, parent: &MarkedStatement) {
        self.file
            .write_all(
                format!(
                    "{}{} [{}, {}]\n",
                    self.prefix(),
                    statement.statement,
                    parent.statement,
                    statement
                        .rule
                        .as_ref()
                        .map(|x| x.to_string())
                        .unwrap_or_default()
                )
                .as_bytes(),
            )
            .expect("Unable write into dump file");
    }
}

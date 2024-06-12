use std::{
    fs::File,
    io::prelude::*,
    path::Path,
    sync::{Arc, Mutex},
};

use serde_derive::Deserialize;

use crate::{
    task::{SolveStatus, Task},
    term::TermProps,
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

pub trait DumperSink: Send + Sync {
    fn subtask_start(&mut self, task: &Task);

    fn subtask_end(&mut self, status: &SolveStatus);

    fn add_term(&mut self, term: &TermProps, parent: &TermProps);
}

pub struct FileDumper {
    subtask_level: usize,
    file:          File,
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
    fn subtask_start(&mut self, task: &Task) {
        self.sink.lock().unwrap().subtask_start(task);
    }

    fn subtask_end(&mut self, status: &SolveStatus) {
        self.sink.lock().unwrap().subtask_end(status);
    }

    fn add_term(&mut self, term: &TermProps, parent: &TermProps) {
        self.sink.lock().unwrap().add_term(term, parent);
    }
}

impl DumperSink for NoneDumper {
    fn subtask_start(&mut self, _: &Task) {}

    fn subtask_end(&mut self, _: &SolveStatus) {}

    fn add_term(&mut self, _: &TermProps, _: &TermProps) {}
}

impl FileDumper {
    pub fn new(file_name: &str) -> FileDumper {
        let path: &Path = file_name.as_ref();
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).expect("unable to create directory");
        }
        FileDumper {
            subtask_level: 0,
            file:          File::create(path).expect("Unable to create dump file"),
        }
    }

    fn prefix(&self) -> String {
        String::from("┆ ").repeat(self.subtask_level)
    }
}

impl DumperSink for FileDumper {
    fn subtask_start(&mut self, task: &Task) {
        self.file
            .write_all(
                format!(
                    "{} {} [{}]\n",
                    self.prefix(),
                    task.purpose,
                    task.conditions
                        .iter()
                        .map(|x| x.term.to_string())
                        .collect::<Vec<String>>()
                        .join(";"),
                )
                .as_bytes(),
            )
            .expect("Unable write into dump file");
        self.subtask_level += 1;
    }

    fn subtask_end(&mut self, status: &SolveStatus) {
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
        self.subtask_level -= 1;
    }

    fn add_term(&mut self, term: &TermProps, parent: &TermProps) {
        self.file
            .write_all(
                format!(
                    "{}{} [{}, {}]\n",
                    self.prefix(),
                    term.term,
                    parent.term,
                    term.rule
                        .as_ref()
                        .map(|x| x.to_string())
                        .unwrap_or_default()
                )
                .as_bytes(),
            )
            .expect("Unable write into dump file");
    }
}

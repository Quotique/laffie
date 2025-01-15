use std::{fs::File, io::prelude::*, path::Path};

use crate::{
    rule::{SharedRule, Suppose},
    task::{Solution, Task},
    term::TermProps,
};

use super::Tracer;

const WRITE_ERROR_TEXT: &str = "Unable write into dump file";

pub struct FileDumpTracer {
    subtask_start_cycle: Vec<usize>,
    file:                File,
}

impl FileDumpTracer {
    pub fn new(file_name: &str) -> FileDumpTracer {
        let path: &Path = file_name.as_ref();
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).expect("unable to create directory");
        }
        FileDumpTracer {
            subtask_start_cycle: Default::default(),
            file:                File::create(path).expect("Unable to create dump file"),
        }
    }

    fn idention(&self) -> String {
        String::from("┆ ").repeat(self.subtask_start_cycle.len())
    }
}

impl Tracer for FileDumpTracer {
    fn on_subtask_start(&mut self, task: &Task, cycle: usize) {
        self.file
            .write_all(
                format!(
                    "{} {} [{}]\n",
                    self.idention(),
                    task.purpose,
                    task.conditions
                        .iter()
                        .map(|x| x.term.to_string())
                        .collect::<Vec<String>>()
                        .join(";"),
                )
                .as_bytes(),
            )
            .expect(WRITE_ERROR_TEXT);
        self.subtask_start_cycle.push(cycle);
    }

    fn on_subtask_end(&mut self, status: &Solution) {
        self.file
            .write_all(
                format!(
                    "{} [{} cycles] {} Answer: {}\n",
                    self.idention(),
                    *status.cycles.as_ref().borrow() -
                        self.subtask_start_cycle
                            .pop()
                            .expect("finished task never starts"),
                    status.task.purpose.to_string().replace("\n", "; "),
                    status
                        .answer()
                        .map(|x| x.to_string())
                        .unwrap_or("not solved".to_owned()),
                )
                .as_bytes(),
            )
            .expect(WRITE_ERROR_TEXT);
    }

    fn on_new_term(&mut self, term: &TermProps, parent: &TermProps) {
        self.file
            .write_all(
                format!(
                    "{}{} [{}, {}]\n",
                    self.idention(),
                    term.term,
                    parent.term,
                    term.rule
                        .as_ref()
                        .map(|x| x.to_string())
                        .unwrap_or_default()
                )
                .as_bytes(),
            )
            .expect(WRITE_ERROR_TEXT);
    }

    fn on_term_focus(&mut self, term: &TermProps) {
        self.file
            .write_all(format!("{}[> {}\n", self.idention(), term).as_bytes())
            .expect(WRITE_ERROR_TEXT);
    }

    fn on_rule_selection(&mut self, rule: SharedRule) {
        self.file
            .write_all(format!("{}>> {}\n", self.idention(), rule).as_bytes())
            .expect(WRITE_ERROR_TEXT);
    }

    fn on_new_suppose(&mut self, rule: SharedRule, suppose: &Suppose) {
        self.file
            .write_all(format!("{}|> {} {}\n", self.idention(), rule, suppose).as_bytes())
            .expect(WRITE_ERROR_TEXT);
    }
}

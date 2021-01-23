use crate::{
    core::symbols::{add_symbol, Symbol},
    parser::{lang, ProblemParser},
    problem::{Problem, ProblemBuilder},
};
use std::{convert::TryFrom, fs, io, path::Path};
use trees::Tree;

pub struct DirectoryParser {
    symbols_path:  String,
    problems_path: String,
}

impl DirectoryParser {
    pub fn new(symbols_path: String, problems_path: String) -> Self {
        Self {
            symbols_path:  symbols_path,
            problems_path: problems_path,
        }
    }

    pub fn load_symbols(&self) -> io::Result<()> {
        Self::load_dir(
            Path::new(self.symbols_path.as_str()),
            &vec!["sym"],
            &mut |s: &Tree<String>| {
                if let Ok(sym) = Symbol::try_from(s) {
                    add_symbol(sym);
                }
            },
        );
        Ok(())
    }

    pub fn load_problems(&self) -> io::Result<Vec<Problem>> {
        let mut result = vec![];
        Self::load_dir(
            Path::new(self.problems_path.as_str()),
            &vec!["pbl"],
            &mut |s| {
                trace!("New problem cb: {}", s);
                if s.root().data == "Problem" {
                    match ProblemParser::with(&s).parse() {
                        Ok(p) => result.push(p),
                        Err(e) => error!("Problem not parsed: {}", e),
                    }
                }
            },
        )?;
        Ok(result)
    }

    fn load_dir<F: FnMut(&Tree<String>)>(
        dir: &Path,
        extensions: &Vec<&str>,
        cb: &mut F,
    ) -> io::Result<()> {
        trace!("Processing dir: {}", dir.to_string_lossy());
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::load_dir(&path, extensions, cb)?;
            } else if path.extension().is_some() &&
                extensions.contains(&path.extension().unwrap().to_str().unwrap())
            {
                Self::load_file(&path, cb)?;
            }
        }
        Ok(())
    }

    fn load_file<F: FnMut(&Tree<String>)>(file: &Path, cb: &mut F) -> io::Result<()> {
        info!("Processing file: {}", file.to_string_lossy());
        let content = fs::read_to_string(file)?;
        let states = lang::StatementsParser::new().parse(&content[..]).unwrap();
        for s in states {
            cb(&s);
        }

        Ok(())
    }
}

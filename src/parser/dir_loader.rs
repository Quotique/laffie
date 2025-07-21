use std::{
    fs, io,
    path::{Path, PathBuf},
};

use itertools::Itertools;
use trees::Tree;

use solver::{
    rule::RulesEngine,
    task::Task,
    term::{Symbol, SymbolProgram},
};

use crate::{lang, NodeData, RuleParser, SymbolParser, TaskParser};

pub struct DirectoryParser {
    symbols_path: PathBuf,
    tasks_path:   PathBuf,
}

impl DirectoryParser {
    pub fn new<P: AsRef<Path>>(symbols_path: P, tasks_path: P) -> Self {
        Self {
            symbols_path: PathBuf::from(symbols_path.as_ref()),
            tasks_path:   PathBuf::from(tasks_path.as_ref()),
        }
    }

    pub fn load_rules(&self) -> io::Result<RulesEngine> {
        info!(target: "init", "Reading rules: {:?}", self.symbols_path);

        self.load_symbols()?;

        let mut result = RulesEngine::default();
        let mut last_sym: Option<Symbol> = None;

        Self::load_dir(
            &self.symbols_path,
            &["sym"],
            &mut |path, src, s: &Tree<NodeData>| {
                if let Ok(sym) = SymbolParser::new(s).parse() {
                    let sym = SymbolProgram::register(sym);
                    last_sym.replace(sym);
                } else if let Ok(rules) = RuleParser::with(s)
                    .with_func_symbol(last_sym.as_ref().unwrap().clone())
                    .parse()
                    .map_err(|e| error!("Rule not parsed: {}", e.error_string(src, Some(path))))
                {
                    for rule in rules {
                        result.register_rule(rule);
                    }
                }
            },
        )?;
        Ok(result)
    }

    pub fn load_tasks(&self) -> io::Result<Vec<Task>> {
        let mut result = vec![];
        Self::load_dir(self.tasks_path.as_ref(), &["pbl"], &mut |path, src, s| {
            if s.root().data().symbol == "Task" {
                match TaskParser::with(s).parse() {
                    Ok(mut t) => {
                        trace!(
                            "New task: [{:x}] {} [{}]",
                            t.id,
                            t.purpose,
                            t.conditions.iter().format(", ")
                        );
                        t.group = path.with_extension("").display().to_string();
                        result.push(t)
                    }
                    Err(e) => error!("Task not parsed: {}", e.error_string(src, Some(path))),
                }
            }
        })?;
        Ok(result)
    }

    fn load_symbols(&self) -> io::Result<()> {
        Self::load_dir(
            &self.symbols_path,
            &["sym"],
            &mut |_, _, s: &Tree<NodeData>| {
                let _ = SymbolParser::new(s).parse().map(|s| s.register());
            },
        )
    }

    fn load_dir<F>(dir: &Path, extensions: &[&str], cb: &mut F) -> io::Result<()>
    where
        F: FnMut(&Path, &str, &Tree<NodeData>),
    {
        trace!("Processing dir: {}", dir.to_string_lossy());
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::load_dir(&path, extensions, cb)?;
            } else if path
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| extensions.contains(&x))
                .unwrap_or(false)
            {
                Self::load_file(&path, cb)?;
            }
        }
        Ok(())
    }

    fn load_file<P, F>(file: P, cb: &mut F) -> io::Result<()>
    where
        P: AsRef<Path>,
        F: FnMut(&Path, &str, &Tree<NodeData>),
    {
        info!("Processing file: {}", file.as_ref().to_string_lossy());
        let content = fs::read_to_string(file.as_ref())?;
        let states = lang::any(content.as_str()).map_err(|e| {
            let error_string = e.error_string(content.as_str(), Some(file.as_ref()));
            error!(
                "Unable to parse file {}: {}",
                file.as_ref().to_string_lossy(),
                error_string
            );
            io::Error::new(io::ErrorKind::InvalidData, error_string)
        })?;
        for s in states {
            cb(file.as_ref(), content.as_str(), &s);
        }

        Ok(())
    }
}

use std::{fs, io, path::Path};

use trees::Tree;

use mcore::{
    predefine::add_symbol, problem::Problem, rule::RulesEngine, statement::symbols::Symbol,
};

use crate::{lang, NodeData, ProblemParser, RuleParser, SymbolParser};

pub struct DirectoryParser {
    symbols_path:  String,
    problems_path: String,
}

impl DirectoryParser {
    pub fn new(symbols_path: String, problems_path: String) -> Self {
        Self {
            symbols_path,
            problems_path,
        }
    }

    pub fn load_rules(&self) -> io::Result<RulesEngine> {
        info!(target: "init", "Reading rules: {:?}", self.symbols_path);

        self.load_symbols()?;

        let mut result = RulesEngine::default();
        let mut last_sym: Option<Symbol> = None;

        Self::load_dir(
            Path::new(self.symbols_path.as_str()),
            &["sym"],
            &mut |src, s: &Tree<NodeData>| {
                if let Ok(sym) = SymbolParser::new(s).parse() {
                    let sym = add_symbol(sym);
                    last_sym.replace(sym);
                } else if let Ok(rules) = RuleParser::with(s)
                    .with_symbol_id(last_sym.as_ref().unwrap().id)
                    .parse()
                    .map_err(|e| error!("Rule not parsed: {}", e.error_string(src)))
                {
                    for rule in rules {
                        trace!("New rule: {}", rule);
                        result.register_raw_rule(rule);
                    }
                }
            },
        )?;
        Ok(result)
    }

    pub fn load_problems(&self) -> io::Result<Vec<Problem>> {
        let mut result = vec![];
        Self::load_dir(
            Path::new(self.problems_path.as_str()),
            &["pbl"],
            &mut |src, s| {
                trace!("New problem cb: {}", s);
                if s.root().data().symbol == "Problem" {
                    match ProblemParser::with(s).parse() {
                        Ok(p) => result.push(p),
                        Err(e) => error!("Problem not parsed: {}", e.error_string(src)),
                    }
                }
            },
        )?;
        Ok(result)
    }

    fn load_symbols(&self) -> io::Result<()> {
        Self::load_dir(
            Path::new(self.symbols_path.as_str()),
            &["sym"],
            &mut |_, s: &Tree<NodeData>| {
                if let Ok(sym) = SymbolParser::new(s).parse() {
                    add_symbol(sym);
                }
            },
        )
    }

    fn load_dir<F: FnMut(&str, &Tree<NodeData>)>(
        dir: &Path,
        extensions: &[&str],
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

    fn load_file<F: FnMut(&str, &Tree<NodeData>)>(file: &Path, cb: &mut F) -> io::Result<()> {
        info!("Processing file: {}", file.to_string_lossy());
        let content = fs::read_to_string(file)?;
        let states = lang::any(content.as_str())
            .map_err(|e| {
                error!(
                    "Unable to parse file {}: {}",
                    file.to_string_lossy(),
                    e.error_string(content.as_str())
                )
            })
            .unwrap();
        for s in states {
            cb(content.as_str(), &s);
        }

        Ok(())
    }
}

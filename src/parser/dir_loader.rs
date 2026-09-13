use std::{
    fs, io,
    path::{Path, PathBuf},
};

use itertools::Itertools;

use solver::{
    rule::{Rule, RulesEngine},
    task::Task,
    term::{Symbol, try_sym},
};

use crate::{
    LoadError, LoadFailure, ParserError, RuleParser, SymbolParser, TaskParser, Tree,
    grammar::{TOKEN_DECLARE, TOKEN_RULE, TOKEN_SYMBOL, TOKEN_TASK},
    lang,
};

/// Parsed value plus every non-fatal error collected during a directory load.
pub struct LoadReport<T> {
    pub value:  T,
    pub errors: Vec<LoadError>,
}

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

    pub fn load_rules(&self) -> Result<RulesEngine, LoadFailure> {
        info!(target: "init", "Reading rules: {:?}", self.symbols_path);

        let (files, mut errors) = Self::gather(&self.symbols_path, &["sym"])?;

        // Pass 1: register every symbol so rules can resolve any of them; the
        // symbol's Python runs here, once.
        for file in &files {
            for block in &file.blocks {
                if block.data().symbol != TOKEN_DECLARE {
                    continue;
                }
                match SymbolParser::from(block).parse() {
                    Ok(program) => {
                        program.register();
                    }
                    Err(e) => errors.push(file.error(&e)),
                }
            }
        }

        // Pass 2: parse rules, each attached to the most recent symbol in its
        // file. Symbols are looked up, not re-parsed (no double Python).
        let mut parsed: Vec<Rule> = Vec::new();
        for file in &files {
            let mut last_sym: Option<Symbol> = None;
            for block in &file.blocks {
                match block.data().symbol.as_str() {
                    TOKEN_DECLARE => last_sym = declare_name(block).and_then(try_sym),
                    TOKEN_RULE => {
                        let Some(func_symbol) = last_sym.clone() else {
                            errors.push(file.error(&ParserError {
                                loc: block.data().location.clone(),
                                msg: "rule appears before any symbol declaration".to_owned(),
                            }));
                            continue;
                        };
                        match RuleParser::from(block)
                            .with_func_symbol(func_symbol)
                            .parse()
                        {
                            Ok(rules) => parsed.extend(rules),
                            Err(e) => errors.push(file.error(&e)),
                        }
                    }
                    _ => {}
                }
            }
        }

        let engine = match parsed.into_iter().collect::<Result<RulesEngine, _>>() {
            Ok(engine) => Some(engine),
            Err(rejected) => {
                errors.extend(rejected.iter().map(|r| LoadError {
                    path:    self.symbols_path.clone(),
                    message: r.to_string(),
                }));
                None
            }
        };

        match engine {
            Some(engine) if errors.is_empty() => Ok(engine),
            _ => Err(LoadFailure::Corpus(errors)),
        }
    }

    pub fn load_tasks(&self) -> io::Result<LoadReport<Vec<Task>>> {
        let (files, mut errors) = Self::gather(&self.tasks_path, &["pbl"])?;

        let mut result = Vec::new();
        for file in &files {
            for block in &file.blocks {
                if block.data().symbol != TOKEN_TASK {
                    continue;
                }
                match TaskParser::from(block).parse() {
                    Ok(mut t) => {
                        trace!(
                            "New task: [{:x}] {} [{}]",
                            t.id,
                            t.goal(),
                            t.givens.iter().format(", ")
                        );
                        t.group = file.path.with_extension("").display().to_string();
                        result.push(t);
                    }
                    Err(e) => errors.push(file.error(&e)),
                }
            }
        }

        Ok(LoadReport {
            value: result,
            errors,
        })
    }

    /// Walk `dir` recursively (sorted), reading and parsing each file once.
    /// Read and parse failures go into the returned errors; only an unreadable
    /// directory is fatal.
    fn gather(dir: &Path, extensions: &[&str]) -> io::Result<(Vec<ParsedFile>, Vec<LoadError>)> {
        trace!("Processing dir: {}", dir.to_string_lossy());
        let mut entries = fs::read_dir(dir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<io::Result<Vec<_>>>()?;
        entries.sort();

        let mut files = Vec::new();
        let mut errors = Vec::new();
        for path in entries {
            if path.is_dir() {
                let (sub_files, sub_errors) = Self::gather(&path, extensions)?;
                files.extend(sub_files);
                errors.extend(sub_errors);
            } else if path
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| extensions.contains(&x))
                .unwrap_or(false)
            {
                info!("Processing file: {}", path.to_string_lossy());
                let src = match fs::read_to_string(&path) {
                    Ok(src) => src,
                    Err(e) => {
                        errors.push(LoadError {
                            message: format!("cannot read file: {e}"),
                            path,
                        });
                        continue;
                    }
                };
                match lang::any(&src) {
                    Ok(blocks) => files.push(ParsedFile { path, src, blocks }),
                    Err(e) => errors.push(LoadError {
                        message: e.error_string(&src, Some(&path)),
                        path,
                    }),
                }
            }
        }
        Ok((files, errors))
    }
}

/// A source file read once, split into its top-level blocks.
struct ParsedFile {
    path:   PathBuf,
    src:    String,
    blocks: Vec<Tree>,
}

impl ParsedFile {
    fn error(&self, e: &ParserError) -> LoadError {
        LoadError {
            path:    self.path.clone(),
            message: e.error_string(&self.src, Some(&self.path)),
        }
    }
}

/// Name of a `Declare` block's symbol, if present.
fn declare_name(block: &Tree) -> Option<&str> {
    block
        .iter()
        .find(|c| c.data().symbol == TOKEN_SYMBOL)
        .and_then(|c| c.front())
        .map(|n| n.data().symbol.as_str())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn a_dangling_block_yields_no_base_at_all() {
        let dir = std::env::temp_dir().join("laffie_dir_loader_dangling");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("t.sym"),
            "symbol qq { attr infix(10); }\n\
             rule {\n\
                 attr level(0),block(nonexistent);\n\
                 qq(x) => x;\n\
             }\n",
        )
        .unwrap();

        // Running against a base that silently dropped this rule would answer
        // by a rule set that is not what the file says.
        let Err(LoadFailure::Corpus(errors)) = DirectoryParser::new(&dir, &dir).load_rules() else {
            panic!("a dangling block must not yield a base");
        };
        assert!(
            errors.iter().any(|e| e.message.contains("nonexistent")),
            "expected the undefined id in the message, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_block_defined_in_a_later_file_still_resolves() {
        let dir = std::env::temp_dir().join("laffie_dir_loader_forward");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // `a.sym` sorts first, so its reference points at a rule not yet read.
        fs::write(
            dir.join("a.sym"),
            "symbol aa { attr infix(10); }\n\
             rule {\n\
                 attr level(0),block(defined_later);\n\
                 aa(x) => x;\n\
             }\n",
        )
        .unwrap();
        fs::write(
            dir.join("b.sym"),
            "symbol bb { attr infix(10); }\n\
             rule {\n\
                 attr level(0),id(defined_later);\n\
                 bb(x) => x;\n\
             }\n",
        )
        .unwrap();

        let engine = DirectoryParser::new(&dir, &dir)
            .load_rules()
            .expect("a forward reference resolves without deferral");
        assert_eq!(engine.iter().count(), 2);

        let _ = fs::remove_dir_all(&dir);
    }
}

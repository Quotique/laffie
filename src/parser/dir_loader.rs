use std::{
    fs, io,
    path::{Path, PathBuf},
};

use itertools::Itertools;

use solver::{
    rule::{Rule, RulesEngine},
    task::Task,
};

use crate::{
    LoadError, LoadFailure, Location, ParserError, RuleParser, SymbolParser, TaskParser, Tree,
    grammar::{TOKEN_DECLARE, TOKEN_RULE, TOKEN_TASK},
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

        // Pass 2: parse rules.
        let mut parsed: Vec<(Rule, (&ParsedFile, &Location))> = Vec::new();
        for file in &files {
            for block in &file.blocks {
                if block.data().symbol != TOKEN_RULE {
                    continue;
                }
                match RuleParser::from(block).parse() {
                    Ok(rules) => {
                        let origin = (file, &block.data().location);
                        parsed.extend(rules.into_iter().map(|r| (r, origin)));
                    }
                    Err(e) => errors.push(file.error(&e)),
                }
            }
        }

        let (rules, origin): (Vec<_>, Vec<_>) = parsed.into_iter().unzip();
        let engine = match rules.into_iter().collect::<Result<RulesEngine, _>>() {
            Ok(engine) => Some(engine),
            Err(rejected) => {
                errors.extend(rejected.iter().map(|(at, r)| {
                    let (file, loc) = origin[*at];
                    file.error(&ParserError {
                        loc: loc.clone(),
                        msg: r.to_string(),
                    })
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Writes `files` into a fresh directory named after `test`.
    fn corpus(test: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("laffie_dir_loader_{test}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for (name, text) in files {
            fs::write(dir.join(name), text).unwrap();
        }
        dir
    }

    fn corpus_errors(dir: &Path) -> Vec<String> {
        let Err(LoadFailure::Corpus(errors)) = DirectoryParser::new(dir, dir).load_rules() else {
            panic!("a rejected rule must not yield a base");
        };
        errors.into_iter().map(|e| e.message).collect()
    }

    #[test]
    fn a_dangling_block_yields_no_base_at_all() {
        let dir = corpus(
            "dangling",
            &[(
                "t.sym",
                "symbol qq { attr infix(10); }\n\
             rule {\n\
                 attr level(0),block(nonexistent);\n\
                 qq(x) => x;\n\
             }\n",
            )],
        );

        // Running against a base that silently dropped this rule would answer
        // by a rule set that is not what the file says.
        let errors = corpus_errors(&dir);
        let at = format!("{}:2:", dir.join("t.sym").display());
        assert!(
            errors
                .iter()
                .any(|e| e.contains("nonexistent") && e.contains(&at)),
            "expected the undefined id at {at}, got: {errors:?}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pattern_without_symbols_names_its_file_and_line() {
        let dir = corpus(
            "no_symbols",
            &[(
                "t.sym",
                "rule {\n\
                 attr level(0);\n\
                 a => b;\n\
             }\n",
            )],
        );

        let errors = corpus_errors(&dir);
        let at = format!("{}:1:", dir.join("t.sym").display());
        assert!(
            errors
                .iter()
                .any(|e| e.contains("no function symbol") && e.contains(&at)),
            "expected the rejection at {at}, got: {errors:?}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_rejected_expansion_points_at_its_own_block() {
        // `one(a)` turns `a * b => a` into a second rule `b => 1`, which has no
        // symbol. The block after it must not take the blame.
        let dir = corpus(
            "expansion",
            &[(
                "t.sym",
                "rule {\n\
                 attr level(0),one(a);\n\
                 a * b => a;\n\
             }\n\
             rule {\n\
                 attr level(0);\n\
                 a + b == 0 => a == -b;\n\
             }\n",
            )],
        );

        let errors = corpus_errors(&dir);
        let path = dir.join("t.sym").display().to_string();
        assert_eq!(errors.len(), 1, "got: {errors:?}");
        assert!(
            errors[0].contains(&format!("{path}:1:")),
            "expected line 1, got: {errors:?}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_with_rules_only_loads() {
        let dir = corpus(
            "rules_only",
            &[
                ("a.sym", "rule {\n attr level(0);\n bb(x) => x;\n}\n"),
                ("b.sym", "symbol bb { attr infix(10); }\n"),
            ],
        );

        let engine = DirectoryParser::new(&dir, &dir)
            .load_rules()
            .expect("symbols resolve across files");
        assert_eq!(engine.iter().count(), 1);

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

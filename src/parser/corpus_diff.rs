//! The whole corpus lowers to the same symbols, rules and tasks through the
//! reference pipeline and the pipeline under change: task ids are computed from
//! the parsed terms, so any drift changes answers' baselines silently.

use std::{
    fs,
    path::{Path, PathBuf},
};

use solver::{
    rule::Rule,
    task::Task,
    term::{SymbolProgram, TermBuf},
};

use crate::{
    CompactString, RuleParser, SymbolParser, TaskParser, Tree,
    grammar::{TOKEN_DECLARE, TOKEN_RULE, TOKEN_TASK},
    lang, legacy,
};

/// Intended differences, as `(file, reason)`; each one needs the user's sign-off.
const ALLOWED: &[(&str, &str)] = &[];

/// One way to turn corpus text into symbols, rules and tasks. A block that
/// fails to parse is kept as its message, so both pipelines must fail alike.
trait Pipeline {
    fn symbols(&self, text: &str) -> Vec<Result<SymbolProgram, String>>;
    fn rules(&self, text: &str) -> Vec<Result<Vec<Rule>, String>>;
    fn tasks(&self, text: &str) -> Vec<Result<Task, String>>;
}

struct Legacy;
struct Current;

#[derive(Debug, PartialEq)]
struct SymbolView {
    name:  CompactString,
    attrs: Vec<String>,
}

#[derive(Debug, PartialEq)]
struct RuleView {
    id:           String,
    level:        String,
    attrs:        Vec<String>,
    block:        String,
    term:         TermBuf,
    pattern:      String,
    replace:      String,
    requirements: Vec<TermBuf>,
}

#[derive(Debug, PartialEq)]
struct TaskView {
    id:      u64,
    name:    String,
    text:    String,
    goal:    TermBuf,
    givens:  Vec<TermBuf>,
    answers: Vec<TermBuf>,
}

impl Pipeline for Legacy {
    fn symbols(&self, text: &str) -> Vec<Result<SymbolProgram, String>> {
        blocks(
            legacy::lang::any(text),
            legacy::grammar::TOKEN_DECLARE,
            |b| legacy::SymbolParser::from(b).parse(),
        )
    }

    fn rules(&self, text: &str) -> Vec<Result<Vec<Rule>, String>> {
        blocks(legacy::lang::any(text), legacy::grammar::TOKEN_RULE, |b| {
            legacy::RuleParser::from(b).parse()
        })
    }

    fn tasks(&self, text: &str) -> Vec<Result<Task, String>> {
        blocks(legacy::lang::any(text), legacy::grammar::TOKEN_TASK, |b| {
            legacy::TaskParser::from(b).parse()
        })
    }
}

impl Pipeline for Current {
    fn symbols(&self, text: &str) -> Vec<Result<SymbolProgram, String>> {
        blocks(lang::any(text), TOKEN_DECLARE, |b| {
            SymbolParser::from(b).parse()
        })
    }

    fn rules(&self, text: &str) -> Vec<Result<Vec<Rule>, String>> {
        blocks(lang::any(text), TOKEN_RULE, |b| RuleParser::from(b).parse())
    }

    fn tasks(&self, text: &str) -> Vec<Result<Task, String>> {
        blocks(lang::any(text), TOKEN_TASK, |b| TaskParser::from(b).parse())
    }
}

#[test]
fn every_corpus_block_lowers_the_same_through_both_pipelines() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root");
    let sym_files = corpus_files(&root.join("symbols"), "sym");
    let task_files = corpus_files(&root.join("tasks"), "pbl");
    assert!(
        !sym_files.is_empty() && !task_files.is_empty(),
        "no corpus under {root:?}"
    );

    let mut diffs = Vec::new();
    // Rules resolve names against every symbol, so all symbols register first.
    for (path, text) in &sym_files {
        let reference = Legacy.symbols(text);
        let current = Current.symbols(text);
        compare(
            path,
            "symbol",
            &reference,
            &current,
            symbol_view,
            &mut diffs,
        );
        for program in current.into_iter().flatten() {
            program.register();
        }
    }
    for (path, text) in &sym_files {
        let (reference, current) = (Legacy.rules(text), Current.rules(text));
        compare(
            path,
            "rule",
            &reference,
            &current,
            |r| rule_view(r),
            &mut diffs,
        );
    }
    for (path, text) in &task_files {
        let (reference, current) = (Legacy.tasks(text), Current.tasks(text));
        compare(path, "task", &reference, &current, task_view, &mut diffs);
    }

    diffs.retain(|d| !ALLOWED.iter().any(|(file, _)| d.contains(file)));
    assert!(
        diffs.is_empty(),
        "{} differences:\n{}",
        diffs.len(),
        diffs.join("\n")
    );
}

/// Every block of `kind`, parsed; a file that does not parse is one failed block.
fn blocks<T, E: ToString>(
    file: Result<Vec<Tree>, crate::ParserError>,
    kind: &str,
    parse: impl Fn(&Tree) -> Result<T, E>,
) -> Vec<Result<T, String>> {
    match file {
        Ok(trees) => trees
            .iter()
            .filter(|b| b.data().symbol == kind)
            .map(|b| parse(b).map_err(|e| e.to_string()))
            .collect(),
        Err(e) => vec![Err(e.to_string())],
    }
}

fn compare<T, V: PartialEq + std::fmt::Debug>(
    path: &Path,
    kind: &str,
    reference: &[Result<T, String>],
    current: &[Result<T, String>],
    view: impl Fn(&T) -> V,
    diffs: &mut Vec<String>,
) {
    let view = |r: &Result<T, String>| r.as_ref().map(&view).map_err(Clone::clone);
    if reference.len() != current.len() {
        diffs.push(format!(
            "{}: {} {kind} blocks vs {}",
            path.display(),
            reference.len(),
            current.len()
        ));
    }
    for (at, (r, c)) in reference.iter().zip(current).enumerate() {
        let (r, c) = (view(r), view(c));
        if r != c {
            diffs.push(format!(
                "{}: {kind} #{at}\n  was: {r:?}\n  now: {c:?}",
                path.display()
            ));
        }
    }
}

fn symbol_view(program: &SymbolProgram) -> SymbolView {
    SymbolView {
        name:  program.name.clone(),
        attrs: sorted(program.attrs.iter().map(|a| format!("{a:?}"))),
    }
}

fn rule_view(rules: &[Rule]) -> Vec<RuleView> {
    rules
        .iter()
        .map(|r| RuleView {
            id:           format!("{:?}", r.id),
            level:        format!("{:?}", r.level),
            // The attribute map iterates in hash order.
            attrs:        sorted(r.attrs.iter().map(|a| format!("{a:?}"))),
            block:        format!("{:?}", r.block),
            term:         r.term.clone(),
            pattern:      format!("{:?}", r.pattern),
            replace:      format!("{:?}", r.replace),
            requirements: r.requirements.clone(),
        })
        .collect()
}

fn task_view(task: &Task) -> TaskView {
    TaskView {
        id:      task.id,
        name:    task.name.clone(),
        text:    task.text.clone(),
        goal:    task.goal().term().as_ref().clone(),
        givens:  task.givens.iter().map(|g| g.as_ref().clone()).collect(),
        answers: task.possible_answers.clone(),
    }
}

fn sorted(items: impl Iterator<Item = String>) -> Vec<String> {
    let mut items: Vec<String> = items.collect();
    items.sort();
    items
}

/// Files with `extension` under `dir`, recursively and in path order, with their text.
fn corpus_files(dir: &Path, extension: &str) -> Vec<(PathBuf, String)> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{dir:?}: {e}"))
        .map(|e| e.expect("a directory entry").path())
        .collect();
    entries.sort();
    let mut files = Vec::new();
    for path in entries {
        if path.is_dir() {
            files.extend(corpus_files(&path, extension));
        } else if path.extension().is_some_and(|x| x == extension) {
            let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
            files.push((path, text));
        }
    }
    files
}

//! Legacy and current pipelines build the corpus identically: task ids hash the
//! parsed terms, so a drift silently moves answer baselines.

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
    CompactString, Tree, legacy,
    source::Source,
    syntax::{Item, grammar::parse},
};

/// Intended differences, as `(file, reason)`; each one needs the user's sign-off.
const ALLOWED: &[(&str, &str)] = &[];

/// A failed item is kept as its message, so both pipelines must fail alike.
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
        legacy_items(
            legacy::lang::any(text),
            legacy::grammar::TOKEN_DECLARE,
            |b| legacy::SymbolParser::from(b).parse(),
        )
    }

    fn rules(&self, text: &str) -> Vec<Result<Vec<Rule>, String>> {
        legacy_items(legacy::lang::any(text), legacy::grammar::TOKEN_RULE, |b| {
            legacy::RuleParser::from(b).parse()
        })
    }

    fn tasks(&self, text: &str) -> Vec<Result<Task, String>> {
        legacy_items(legacy::lang::any(text), legacy::grammar::TOKEN_TASK, |b| {
            legacy::TaskParser::from(b).parse()
        })
    }
}

impl Pipeline for Current {
    fn symbols(&self, text: &str) -> Vec<Result<SymbolProgram, String>> {
        current_items(text, |i, src| match i {
            Item::Symbol(s) => Some(s.build(src)),
            _ => None,
        })
    }

    fn rules(&self, text: &str) -> Vec<Result<Vec<Rule>, String>> {
        current_items(text, |i, src| match i {
            Item::Rule(r) => Some(r.build(src)),
            _ => None,
        })
    }

    fn tasks(&self, text: &str) -> Vec<Result<Task, String>> {
        current_items(text, |i, src| match i {
            Item::Task(t) => Some(t.build(src)),
            _ => None,
        })
    }
}

#[test]
fn every_corpus_item_is_the_same_through_both_pipelines() {
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
        let legacy = Legacy.symbols(text);
        let current = Current.symbols(text);
        compare(path, "symbol", &legacy, &current, symbol_view, &mut diffs);
        for program in current.into_iter().flatten() {
            program.register();
        }
    }
    for (path, text) in &sym_files {
        let (legacy, current) = (Legacy.rules(text), Current.rules(text));
        compare(
            path,
            "rule",
            &legacy,
            &current,
            |r| rule_view(r),
            &mut diffs,
        );
    }
    for (path, text) in &task_files {
        let (legacy, current) = (Legacy.tasks(text), Current.tasks(text));
        compare(path, "task", &legacy, &current, task_view, &mut diffs);
    }

    diffs.retain(|d| !ALLOWED.iter().any(|(file, _)| d.contains(file)));
    assert!(
        diffs.is_empty(),
        "{} differences:\n{}",
        diffs.len(),
        diffs.join("\n")
    );
}

fn legacy_items<T, E: ToString>(
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

fn current_items<T>(
    text: &str,
    build: impl Fn(&Item, &Source) -> Option<Result<T, crate::ParserError>>,
) -> Vec<Result<T, String>> {
    let src = Source::new(text);
    match parse::file(text) {
        Ok(items) => items
            .iter()
            .filter_map(|i| build(i, &src))
            .map(|r| r.map_err(|e| e.to_string()))
            .collect(),
        Err(e) => vec![Err(crate::ParserError::from(e).to_string())],
    }
}

fn compare<T, V: PartialEq + std::fmt::Debug>(
    path: &Path,
    kind: &str,
    legacy: &[Result<T, String>],
    current: &[Result<T, String>],
    view: impl Fn(&T) -> V,
    diffs: &mut Vec<String>,
) {
    let view = |r: &Result<T, String>| r.as_ref().map(&view).map_err(Clone::clone);
    if legacy.len() != current.len() {
        diffs.push(format!(
            "{}: {} {kind} items vs {}",
            path.display(),
            legacy.len(),
            current.len()
        ));
    }
    for (at, (l, c)) in legacy.iter().zip(current).enumerate() {
        let (l, c) = (view(l), view(c));
        if l != c {
            diffs.push(format!(
                "{}: {kind} #{at}\n  was: {l:?}\n  now: {c:?}",
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

use std::{collections::HashSet, process::ExitCode};

use colored::*;

use parser::LoadReport;
use solver::{
    rule::{Rule, RulesEngine},
    task::Task,
    term::symbol_names,
};

/// Lints the loaded corpus. Load errors are hard; suspicious params are
/// warnings (errors under `strict`). Returns the process exit code.
pub fn run(
    rules: &LoadReport<RulesEngine>,
    tasks: &LoadReport<Vec<Task>>,
    strict: bool,
) -> ExitCode {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Parse failures and dangling block(...) refs (loader collects both).
    for e in rules.errors.iter().chain(tasks.errors.iter()) {
        errors.push(format!("{}: {}", e.path.display(), e.message));
    }

    // A param close to a symbol name is likely a mistyped symbol.
    let symbols: Vec<String> = symbol_names()
        .into_iter()
        .map(|s| s.to_string())
        .filter(|s| is_word(s))
        .collect();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    for rule in rules.value.iter() {
        for param in rule_param_names(&rule) {
            if let Some(sym) = typo_of(&param, &symbols) &&
                seen.insert((param.clone(), sym.clone()))
            {
                warnings.push(format!(
                    "param `{param}` looks like symbol `{sym}` (in `{}`)",
                    rule.pattern_node()
                ));
            }
        }
    }

    report(&errors, &warnings);
    if errors.is_empty() && (!strict || warnings.is_empty()) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn rule_param_names(rule: &Rule) -> Vec<String> {
    let mut out = Vec::new();
    for node in [rule.pattern_node(), rule.replace_node()] {
        for v in node.bfs() {
            if let Some(p) = v.data.param() {
                out.push(p.as_ref().to_string());
            }
        }
    }
    out
}

/// The symbol name `param` is a likely typo of, if any: at most two edits and
/// no more than half the shorter word (so `lin` vs `find` is not flagged).
fn typo_of(param: &str, symbols: &[String]) -> Option<String> {
    if !is_word(param) {
        return None;
    }
    let plen = param.chars().count();
    symbols
        .iter()
        .find(|s| {
            if param == s.as_str() {
                return false;
            }
            let d = levenshtein(param, s);
            d <= 2 && 2 * d <= plen.min(s.chars().count())
        })
        .cloned()
}

/// Alphabetic, ≥3 chars — skips single-letter params and operator symbols.
fn is_word(s: &str) -> bool {
    s.chars().count() >= 3 && s.chars().all(|c| c.is_alphabetic())
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn report(errors: &[String], warnings: &[String]) {
    for e in errors {
        println!("{} {e}", "error:".red().bold());
    }
    for w in warnings {
        println!("{} {w}", "warning:".yellow().bold());
    }
    if errors.is_empty() && warnings.is_empty() {
        println!("{}", "check: no issues".green());
    } else {
        println!(
            "check: {} error(s), {} warning(s)",
            errors.len(),
            warnings.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("know", "known"), 1);
        assert_eq!(levenshtein("set", "set"), 0);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn typo_detection() {
        let syms = vec!["known".to_owned(), "set".to_owned(), "find".to_owned()];
        // A one-edit typo of a real symbol is flagged.
        assert_eq!(typo_of("know", &syms), Some("known".to_owned()));
        // An exact symbol name is not a typo.
        assert_eq!(typo_of("set", &syms), None);
        // Single-letter params (the common legit case) are ignored.
        assert_eq!(typo_of("a", &syms), None);
        // A genuine, distinct param name is left alone.
        assert_eq!(typo_of("alpha", &syms), None);
        // Two edits over a 3-char word (`lin` vs `find`) is too loose to flag.
        assert_eq!(typo_of("lin", &syms), None);
    }
}

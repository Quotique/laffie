//! A condition without unknowns is decided on its own, not by the task's assumptions.

use std::{fs, sync::Arc, time::Duration};

use parser::DirectoryLoader;
use solver::engine::{Limits, Solver, TracerHub};

/// The answer to the single task in `src`, solved against the real corpus.
fn answer(name: &str, src: &str) -> Option<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = std::env::temp_dir().join(format!("laffie_closed_conditions_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("t.pbl"), src).unwrap();

    let parser = DirectoryLoader::new(root.join("symbols"), dir.clone());
    let rules = parser.load_rules().expect("the symbols corpus loads clean");
    let task = parser.load_tasks().unwrap().value.pop().expect("one task");
    let (limits, _cancel) = Limits::init(10_000, Duration::from_secs(60));
    let solution = Solver::new(Arc::new(rules)).solve(task, TracerHub::default(), limits);

    let _ = fs::remove_dir_all(&dir);
    solution.answer().map(|a| a.term().to_string())
}

#[test]
fn a_false_condition_does_not_prove_itself() {
    let answer = answer("false", "task { goal find(x); sqrt(11.5) >= 4 && x == 2; }");
    assert_ne!(answer.as_deref(), Some("x=2"), "√11.5 ≥ 4 is false");
}

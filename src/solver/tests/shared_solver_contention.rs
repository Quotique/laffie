//! Falsifier for sharing one `Solver` across threads, the way
//! `src/bin/tui/ui.rs` does.
//!
//! The cycle count is the probe. Deterministic in the search path, so any run
//! state surviving one `solve` into the next moves it.
//!
//! Run with `--nocapture` for the per-probe summaries.

use std::{sync::Arc, thread, time::Duration};

use parser::DirectoryParser;
use solver::{
    engine::{Limits, SolutionStatus, Solver, TracerHub},
    task::Task,
};

const THREADS: usize = 16;
const CYCLE_BUDGET: usize = 100_000;

/// The whole observable outcome of one solve.
type Print = (String, usize);

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root above src/solver")
}

fn corpus() -> (Arc<Solver>, Vec<Task>) {
    let root = repo_root();
    let parser = DirectoryParser::new(root.join("symbols"), root.join("tasks/regress"));

    let rules = parser.load_rules().expect("symbols dir readable");
    assert!(
        rules.errors.is_empty(),
        "{} rule load error(s)",
        rules.errors.len()
    );
    let tasks = parser.load_tasks().expect("tasks dir readable");
    assert!(
        tasks.errors.is_empty(),
        "{} task load error(s)",
        tasks.errors.len()
    );
    assert!(!tasks.value.is_empty(), "the regress corpus is empty");

    (Arc::new(Solver::new(Arc::new(rules.value))), tasks.value)
}

fn solve_once(solver: &Solver, task: &Task) -> Print {
    let (limits, _cancel) = Limits::init(CYCLE_BUDGET, Duration::from_secs(600));
    let solution = solver.solve(task.clone(), TracerHub::default(), limits);
    let status = match &solution.status {
        SolutionStatus::NotDone => "not-done".to_owned(),
        SolutionStatus::Answered(a) => format!("answered:{}", a.term()),
        SolutionStatus::Err(e) => format!("err:{e}"),
    };
    (status, solution.cycles())
}

/// Keyed by position, because the corpus does not give tasks unique names.
fn whole_corpus(solver: &Solver, tasks: &[Task]) -> Vec<Print> {
    tasks.iter().map(|t| solve_once(solver, t)).collect()
}

/// Three passes over one `Solver`. State kept there would make the later ones
/// cheaper.
#[test]
fn repeated_passes_on_one_solver_agree() {
    let (solver, tasks) = corpus();
    let first = whole_corpus(&solver, &tasks);
    let second = whole_corpus(&solver, &tasks);
    let third = whole_corpus(&solver, &tasks);

    assert_eq!(first, second, "pass 2 diverged from pass 1");
    assert_eq!(first, third, "pass 3 diverged from pass 1");
    println!(
        "A: {} tasks, 3 sequential passes on one Solver, identical, {} cycles per pass",
        first.len(),
        first.iter().map(|(_, c)| c).sum::<usize>()
    );
}

/// Every thread solves the whole corpus at once through one `Arc<Solver>`.
#[test]
fn every_thread_reproduces_the_sequential_run() {
    let (solver, tasks) = corpus();
    let reference = whole_corpus(&solver, &tasks);

    let start = std::time::Instant::now();
    let results: Vec<Vec<Print>> = thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let solver = Arc::clone(&solver);
                let tasks = tasks.clone();
                scope.spawn(move || whole_corpus(&solver, &tasks))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("no panic in a solver thread"))
            .collect()
    });
    let wall = start.elapsed();

    for (n, got) in results.iter().enumerate() {
        assert_eq!(got, &reference, "thread {n} diverged from sequential");
    }
    println!(
        "B: {THREADS} threads x {} tasks on one Arc<Solver>, all identical to sequential, \
         wall {:.2}s",
        reference.len(),
        wall.as_secs_f64()
    );
}

/// Every thread on the same task at once, so a shared table takes every write
/// in one cycle window.
#[test]
fn one_task_under_every_thread_at_once_agrees() {
    let (solver, tasks) = corpus();

    for task in &tasks {
        let expected = solve_once(&solver, task);
        let got: Vec<Print> = thread::scope(|scope| {
            let handles: Vec<_> = (0..THREADS)
                .map(|_| {
                    let solver = Arc::clone(&solver);
                    let task = task.clone();
                    scope.spawn(move || solve_once(&solver, &task))
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("no panic in a solver thread"))
                .collect()
        });
        for (n, g) in got.iter().enumerate() {
            assert_eq!(g, &expected, "task {}: thread {n} diverged", task.name);
        }
    }
    println!(
        "C: {} tasks, each under {THREADS} threads at once, all identical",
        tasks.len()
    );
}

#![allow(clippy::redundant_field_names)]
#![allow(clippy::module_inception)]

mod check;
mod coverage;
mod explain;
mod run_diff;
mod settings;

use std::{
    collections::HashMap, convert::TryFrom, fmt, path::PathBuf, process::ExitCode, sync::Arc,
};

use clap::Parser;
use colored::*;
use itertools::Itertools;

use database::{Db, Run, Task as DbTask, id_from_hex, id_to_hex};
use parser::DirectoryParser;
use solver::task::{RunControl, SolutionStatus, Solver, Task as SolverTask, TracerHub};
use view::View;

use crate::{
    coverage::CoverageTracer, explain::ExplainTracer, run_diff::RunDiff, settings::Settings,
};

/// Core develop/debug environment
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    #[clap(short, long, default_value = "config/cli.yaml")]
    config: PathBuf,

    /// Runs only the specified task(s): comma-separated ids or hex prefixes
    #[clap(short, long, default_value = "")]
    only: String,

    /// Remove task from db
    #[clap(short, long, default_value = "")]
    remove: String,

    /// Specify symbols path
    #[clap(short, long)]
    symbols: Option<PathBuf>,

    /// Specify tasks path
    #[clap(short = 'p', long)]
    tasks: Option<PathBuf>,

    /// Specify DB file path
    #[clap(short = 'd', long, default_value = "./db/tasks.redb")]
    db_path: PathBuf,

    /// Dump solution trace into a file
    #[clap(short, long)]
    trace: bool,

    /// Execution deadline (in cycles) for individual problem
    #[clap(short, long, default_value = "100000")]
    exec_deadline: usize,

    /// Wall-clock time limit (in seconds) per problem
    #[clap(short = 'l', long, default_value = "86400")]
    time_limit: u64,

    /// Persist each run to the DB as the new baseline. Off (default) is
    /// read-only: the run-vs-last-run diff is still computed and printed.
    #[clap(long)]
    record: bool,

    /// Lint the corpus (parse errors, dangling block refs, suspicious params,
    /// term roundtrip) instead of solving; exit non-zero on errors.
    #[clap(long)]
    check: bool,

    /// With --check, treat warnings as errors.
    #[clap(long)]
    strict: bool,

    /// Print a per-task breakdown of where the solve spent its cycles
    /// (focused terms, rule accept/reject counts, subtasks). Best with --only.
    #[clap(long)]
    explain: bool,

    /// After the run, list loaded rules that produced no accepted hypothesis
    /// over the tasks run (dead rules), split into never-fired and fired-but-
    /// useless.
    #[clap(long)]
    coverage: bool,
}

#[derive(Clone, Debug, Default)]
struct SolveStats {
    solved:         usize,
    not_solved:     usize,
    answer_changed: usize,
}

impl fmt::Display for SolveStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}, {}: {}, {}: {}",
            "solved".bold().green(),
            self.solved,
            "not solved".bold().yellow(),
            self.not_solved,
            "wrong answer".bold().red(),
            self.answer_changed
        )
    }
}

fn main() -> ExitCode {
    let args = Args::parse();
    let only = args.only;

    let settings = match Settings::new(args.config) {
        Ok(settings) => settings,
        Err(e) => {
            println!("Config error: {e:?}");
            return ExitCode::FAILURE;
        }
    };
    let _log_guard = settings.logger.init();

    let Some(symbols_dir) = args.symbols.clone().or(settings.symbols_dir) else {
        println!("Symbols dir is not specified");
        return ExitCode::FAILURE;
    };
    let Some(tasks_dir) = args.tasks.clone().or(settings.tasks_dir) else {
        println!("Tasks dir is not specified");
        return ExitCode::FAILURE;
    };
    let parser = DirectoryParser::new(symbols_dir, tasks_dir);

    let rules = match parser.load_rules() {
        Ok(report) => report,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let tasks = match parser.load_tasks() {
        Ok(report) => report,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    if args.check {
        return check::run(&rules, &tasks, args.strict);
    }

    // A broken corpus is not a valid run: report every load error and bail out.
    let load_errors: Vec<_> = rules.errors.iter().chain(tasks.errors.iter()).collect();
    if !load_errors.is_empty() {
        for e in &load_errors {
            eprintln!("{}: {}", e.path.display(), e.message);
        }
        eprintln!("{} load error(s)", load_errors.len());
        return ExitCode::FAILURE;
    }
    let rules_engine = Arc::new(rules.value);
    let tasks = tasks.value;

    if let Some(parent) = args.db_path.parent() &&
        !parent.as_os_str().is_empty() &&
        let Err(e) = std::fs::create_dir_all(parent)
    {
        println!("Cannot create db parent dir: {e}");
        return ExitCode::FAILURE;
    }
    let db = match Db::open(&args.db_path) {
        Ok(db) => db,
        Err(e) => {
            println!("Db open error: {e:?}");
            return ExitCode::FAILURE;
        }
    };

    if !args.remove.is_empty() {
        let id = id_from_hex(&args.remove).expect("bad task id");
        if let Err(e) = db.remove_task(id) {
            println!("task remove error: {e:?}");
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    let mut stats: HashMap<String, SolveStats> = Default::default();
    let mut diff = RunDiff::default();
    let coverage = args.coverage.then(CoverageTracer::default);

    let only_ids: Vec<&str> = only
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    for solver_task in tasks.into_iter().filter(|task| {
        if only_ids.is_empty() {
            return true;
        }
        if only_ids.iter().any(|w| task.name == *w) {
            return true;
        }
        let id = id_to_hex(&database::compute_task_id(
            &task
                .givens
                .iter()
                .map(|t| (*t.term).clone())
                .collect::<Vec<_>>(),
            &task.goal().term,
        ));
        only_ids
            .iter()
            .any(|w| id.starts_with(w) || id.ends_with(w))
    }) {
        let db_task = DbTask::from(&solver_task);
        if args.record &&
            let Err(e) = db.put_task(&db_task)
        {
            println!("task put error: {e:?}");
        }

        // CONTEXT: dead for a task that came from the loader — its goal was
        // parsed there — and live for one read straight from the db.
        let solver_task: SolverTask = match db_task.clone().try_into() {
            Ok(task) => task,
            Err(e) => {
                println!("{} {}: {e}", "Task".bold().red(), id_to_hex(&db_task.id));
                stats.entry(db_task.group.clone()).or_default().not_solved += 1;
                continue;
            }
        };
        println!("{} {}", "Task".bold().green(), solver_task);
        let task_id_hex = id_to_hex(&db_task.id);
        let task_label = format!("{}/{}", db_task.group, &task_id_hex[..8]);
        let mut solver = Solver::new(rules_engine.clone());

        let explain = args.explain.then(ExplainTracer::default);
        let start = std::time::Instant::now();
        let solution = solver.solve(
            solver_task,
            {
                let mut tracer = TracerHub::default();
                if args.trace {
                    tracer.add_file_dumper(format!("dumps/{task_id_hex}.dump"));
                }
                if let Some(explain) = &explain {
                    tracer.add_custom(explain.clone());
                }
                if let Some(coverage) = &coverage {
                    tracer.add_custom(coverage.clone());
                }
                tracer
            },
            RunControl::init(
                args.exec_deadline,
                std::time::Duration::from_secs(args.time_limit),
            )
            .0,
        );
        let duration_ms = start.elapsed().as_millis() as u64;
        if let Some(explain) = &explain {
            println!("{}", explain.report());
        }
        match solution.status {
            SolutionStatus::Answer(_) => {
                println!(
                    "{}\n{}",
                    "Solution:".italic().blue(),
                    View::try_from(solution.as_ref()).unwrap()
                );
                if solution.validate_answer() {
                    stats.entry(db_task.group.clone()).or_default().solved += 1;
                } else {
                    stats
                        .entry(db_task.group.clone())
                        .or_default()
                        .answer_changed += 1;
                    println!(
                        "{}\nValid answers: [{}]\nObtained: {}",
                        "Answer changed: ".bold().blink().red(),
                        solution.task.possible_answers.iter().format(", "),
                        solution.answer().unwrap()
                    );
                }
            }
            SolutionStatus::Err(e) => {
                stats.entry(db_task.group.clone()).or_default().not_solved += 1;
                println!(
                    "{} {}\n{}",
                    "Solution:".italic().blue(),
                    e.to_string().red(),
                    View::try_from(solution.as_ref()).unwrap()
                );
            }
            SolutionStatus::NotDone => {
                stats.entry(db_task.group.clone()).or_default().not_solved += 1;
                println!(
                    "{} {}\n{}",
                    "Solution:".italic().blue(),
                    "not done".yellow(),
                    View::try_from(solution.as_ref()).unwrap()
                );
            }
        };

        // Diff against the last stored run (read-only; always computed).
        let prev = db.last_run(db_task.id).ok().flatten();
        diff.record(
            &task_label,
            prev.as_ref(),
            solution.answer().as_deref(),
            duration_ms,
        );

        if args.record {
            let mut run = Run::from_solution(db_task.id, &solution);
            run.stats.duration_ms = Some(duration_ms);
            if let Err(e) = db.add_run(run) {
                println!("run add error: {e:?}");
            }
        }
    }

    let mut total: SolveStats = Default::default();
    for (group, stats) in stats.iter() {
        total.solved += stats.solved;
        total.not_solved += stats.not_solved;
        total.answer_changed += stats.answer_changed;
        println!("{group}: {stats}");
    }
    println!("total: {total}");
    println!("{diff}");
    if let Some(coverage) = &coverage {
        println!("{}", coverage.report(&rules_engine));
    }

    // A wrong answer (vs expected) or a regression (newly failing vs last run)
    // is a hard failure; unsolved tasks on their own are not.
    if total.answer_changed > 0 || diff.has_regression() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

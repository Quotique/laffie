#![allow(clippy::redundant_field_names)]
#![allow(clippy::module_inception)]

mod settings;

use std::{
    collections::HashMap, convert::TryFrom, fmt, path::PathBuf, process::ExitCode, sync::Arc,
};

use clap::Parser;
use colored::*;
use itertools::Itertools;

use database::{Db, Run, Task as DbTask, id_from_hex, id_to_hex};
use parser::DirectoryParser;
use solver::task::{SolutionStatus, Solver, Task as SolverTask, TracerHub};
use view::View;

use crate::settings::Settings;

/// Core develop/debug environment
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    #[clap(short, long, default_value = "./config/local.json")]
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

    let Ok(rules_engine) = parser
        .load_rules()
        .map(Arc::new)
        .inspect_err(|e| eprintln!("{e}"))
    else {
        return ExitCode::FAILURE;
    };
    let Ok(tasks) = parser.load_tasks().inspect_err(|e| eprintln!("{e}")) else {
        return ExitCode::FAILURE;
    };

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
            &task.goal.term,
        ));
        only_ids
            .iter()
            .any(|w| id.starts_with(w) || id.ends_with(w))
    }) {
        let db_task = DbTask::from(&solver_task);
        if let Err(e) = db.put_task(&db_task) {
            println!("task put error: {e:?}");
        }

        let solver_task: SolverTask = db_task.clone().into();
        println!("{} {}", "Task".bold().green(), solver_task);
        let task_id_hex = id_to_hex(&db_task.id);
        let mut solver = Solver::new(rules_engine.clone());

        let solution = solver.solve(
            solver_task,
            {
                let mut tracer = TracerHub::default();
                if args.trace {
                    tracer.add_file_dumper(format!("dumps/{task_id_hex}.dump"));
                }
                tracer
            },
            args.exec_deadline,
            std::time::Duration::from_secs(args.time_limit),
        );
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

        let run = Run::from_solution(db_task.id, &solution);
        if let Err(e) = db.add_run(run) {
            println!("run add error: {e:?}");
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

    // Only a wrong answer is a hard failure; unsolved tasks are not.
    if total.answer_changed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#![allow(clippy::redundant_field_names)]
#![allow(clippy::module_inception)]

mod settings;

use std::{collections::HashMap, convert::TryFrom, fmt, path::PathBuf, sync::Arc};

use clap::Parser;
use colored::*;
use itertools::Itertools;

use database::{TaskDb, TaskRecord};
use parser::DirectoryParser;
use solver::task::{SolutionStatus, Solver, Task, TracerHub};
use view::View;

use crate::settings::Settings;

/// Core develop/debug environment
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    #[clap(short, long, default_value = "./config/local.json")]
    config: PathBuf,

    /// Runs only specified task
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

    /// Specify tasks DB path
    #[clap(short = 'd', long, default_value = "./db/tasks")]
    tasks_db: PathBuf,

    /// Dump solution trace into a file
    #[clap(short, long)]
    trace: bool,

    /// Execution deadline (in cycles) for individual problem
    #[clap(short, long, default_value = "100000")]
    exec_deadline: usize,
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

fn main() {
    let args = Args::parse();
    let only = args.only;

    let settings = Settings::new(args.config)
        .map_err(|e| {
            println!("Config error: {e:?}");
            e
        })
        .unwrap_or_else(|_| {
            std::process::exit(-1);
        });
    let _log_guard = settings.logger.init();

    let parser = DirectoryParser::new(
        args.symbols
            .clone()
            .or(settings.symbols_dir)
            .unwrap_or_else(|| {
                println!("Symbols dir is not specified");
                std::process::exit(-1);
            }),
        args.tasks
            .clone()
            .or(settings.tasks_dir)
            .unwrap_or_else(|| {
                println!("Tasks dir is not specified");
                std::process::exit(-1);
            }),
    );

    let Ok(rules_engine) = parser
        .load_rules()
        .map(Arc::new)
        .inspect_err(|e| eprintln!("{e}"))
    else {
        return;
    };
    let Ok(tasks) = parser.load_tasks().inspect_err(|e| eprintln!("{e}")) else {
        return;
    };
    let db = TaskDb::open(args.tasks_db).unwrap();

    if !args.remove.is_empty() {
        let id = i128::from_str_radix(&args.remove, 16).expect("bad task id");
        if let Err(e) = db.remove(id) {
            println!("task remove error: {e}");
        }
        return;
    }

    // TODO: db is temporary disabled
    // for record in tasks.into_iter().map(|p| TaskRecord::from(&p)) {
    //     if let Err(e) = db.get_or_insert(record) {
    //         println!("task write error: {}", e);
    //     }
    // }
    // let db_tasks_iter = Box::new(db.iter());
    // db_tasks_iter
    let mut stats: HashMap<String, SolveStats> = Default::default();

    for mut record in tasks
        .into_iter()
        .map(|p| TaskRecord::from(&p))
        .filter(move |x| {
            let id = format!("{:x}", x.id);
            id.starts_with(&only) || id.ends_with(&only)
        })
    {
        let p: Task = record.clone().into();

        println!("{} {}", "Task".bold().green(), p);
        let p_id = p.id;
        let mut solver = Solver::new(rules_engine.clone());

        let solution = solver.solve(
            p,
            {
                let mut tracer = TracerHub::default();
                if args.trace {
                    tracer.add_file_dumper(format!("dumps/{p_id:x}.dump"));
                }
                tracer
            },
            args.exec_deadline,
        );
        match solution.status {
            SolutionStatus::Answer(_) => {
                stats.entry(record.group.clone()).or_default().solved += 1;
                println!(
                    "{}\n{}",
                    "Solution:".italic().blue(),
                    View::try_from(solution.as_ref()).unwrap()
                );
                if !solution.validate_answer() {
                    stats
                        .entry(record.group.clone())
                        .or_default()
                        .answer_changed += 1;
                    println!(
                        "{}\nValid answers: [{}]\nObtained: {}",
                        "Answer changed: ".bold().blink().red(),
                        solution.task.possible_answers.iter().format(", "),
                        solution.answer().unwrap()
                    );
                }
                record.runs.push(solution.cycles());

                // TODO: db is temporary disabled
                // if let Err(e) = db.put(&record) {
                //     println!("Cant put record {}", e);
                // }
            }
            SolutionStatus::Err(e) => {
                stats.entry(record.group.clone()).or_default().not_solved += 1;
                println!(
                    "{} {}\n{}",
                    "Solution:".italic().blue(),
                    e.to_string().red(),
                    View::try_from(solution.as_ref()).unwrap()
                );
            }
            _ => unreachable!(),
        };
        if !record.reports.is_empty() {
            println!(
                "{} [{}]",
                "Reported:".bold().blink().red(),
                record.reports.iter().format(", ")
            );
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
}

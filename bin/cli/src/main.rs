#![allow(clippy::redundant_field_names)]
#![allow(clippy::module_inception)]

mod settings;

use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use colored::*;

use database::{ProblemDb, ProblemRecord};
use mcore::{
    problem::{Problem, Solution},
    utils::{log_init, Dumper, DumperConfig, VecDisplay},
};
use parser::DirectoryParser;
use view::Console;

use crate::settings::Settings;

/// Minerva core develop/debug enviroment
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    #[clap(short, long, default_value = "./config/local.json")]
    config: PathBuf,

    /// Runs only spcified problem
    #[clap(short, long, default_value = "")]
    only: String,

    /// Specify symbols path
    #[clap(short, long)]
    symbols: Option<PathBuf>,

    /// Specify problems path
    #[clap(short, long)]
    problems: Option<PathBuf>,

    /// Specify problems DB path
    #[clap(short = 'd', long)]
    problems_db: Option<PathBuf>,

    /// Dump solution trace into a file
    #[clap(short, long)]
    trace: bool,
}

fn main() {
    let args = Args::parse();
    let only = args.only;

    let settings = Settings::new(args.config)
        .map_err(|e| {
            println!("Config error: {:?}", e);
            e
        })
        .unwrap_or_else(|_| {
            std::process::exit(-1);
        });
    log_init(&settings.logger);

    let parser = DirectoryParser::new(
        args.symbols
            .clone()
            .or(settings.symbols_dir)
            .unwrap_or_else(|| {
                println!("Symbols dir is not specified");
                std::process::exit(-1);
            }),
        args.problems
            .clone()
            .or(settings.problems_dir)
            .unwrap_or_else(|| {
                println!("Problems dir is not specified");
                std::process::exit(-1);
            }),
    );

    let rules_engine = Arc::new(parser.load_rules().unwrap());
    let problems = parser.load_problems().unwrap();
    let db = args.problems_db.map(|x| ProblemDb::open(x).unwrap());

    fn db_ext<'a>(db: &'a Option<ProblemDb>) -> Box<dyn Iterator<Item = ProblemRecord> + 'a> {
        if let Some(db) = db {
            Box::new(db.iter())
        } else {
            Box::new(std::iter::empty())
        }
    }
    let db_problems_iter = db_ext(&db);

    for record in problems
        .into_iter()
        .map(|p| ProblemRecord::from(&p))
        .chain(db_problems_iter)
        .filter(move |x| {
            if !only.is_empty() {
                let id = format!("{:x}", x.id);
                return id.starts_with(&only) || id.ends_with(&only);
            }
            true
        })
    {
        let p: Problem = record.clone().into();

        println!("{} {}", "Problem".bold().green(), p);
        let p_id = p.id;
        let mut solution = Solution::new(
            p,
            rules_engine.clone(),
            Dumper::new(DumperConfig {
                sink:     if args.trace {
                    "file".into()
                } else {
                    "none".into()
                },
                filename: format!("dumps/{:x}.dump", p_id),
            }),
        );

        match solution.solve() {
            Ok(_) => {
                println!("{} {}", "Solution:".italic().blue(), Console(&solution));
                if let Some(prev) = record.runs.last() {
                    if let (Ok(prev_answer), Ok(answer)) =
                        (&prev.status, &solution.perf_stats.status)
                    {
                        if answer != prev_answer {
                            println!(
                                "{}\nOld:{}\nNew:{}",
                                "Answer changed: ".bold().blink().red(),
                                prev_answer,
                                answer
                            );
                        }
                    }
                }
            }
            Err(e) => {
                println!(
                    "{} {} {}",
                    "Solution:".italic().blue(),
                    e.to_string().red(),
                    Console(&solution)
                );
            }
        };
        if !record.reports.is_empty() {
            println!(
                "{} {}",
                "Reported: ".bold().blink().red(),
                VecDisplay(&record.reports)
            );
        }
    }
}

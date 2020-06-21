extern crate bigdecimal;
extern crate chrono;
extern crate colored;
extern crate config;
extern crate fern;
extern crate multi_map;
extern crate num;
extern crate num_bigint;
extern crate serde;
extern crate trees;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod core;
mod parser;
mod settings;
mod solver;

use std::{env, path::Path, str::FromStr, sync::Arc};

use colored::*;

use core::{rule::RulesEngine, symbols::load_symbols};
use settings::{Logger, Settings};
use solver::{problem::ProblemStorage, solution::Solution};

fn log_init(config: &Logger) {
    let log_level = log::LevelFilter::from_str(&config.level).unwrap_or(log::LevelFilter::Debug);
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Utc::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log_level)
        //.chain(std::io::stdout())
        .chain(fern::log_file(&config.filename).unwrap())
        .apply()
        .unwrap();

    info!(target: "init", "Log initialized with params: {:?}", config);
    info!(target: "init", "Current log level: {:?}", log_level);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let settings = Settings::new();
    match settings {
        Ok(ref s) => log_init(&s.logger),
        Err(e) => println!("Config error: {:?}", e),
    }

    info!(target: "init", "Args {:?}", args);

    if args.len() < 3 {
        println!("Usage: {} <code_dir> <problems_dir>", &args[0]);
        return;
    }
    let code_dir = Path::new(&args[1][..]);
    let problems_dir = Path::new(&args[2][..]);

    info!(target: "init", "Reading symbols: {:?}", code_dir);
    load_symbols(&code_dir).unwrap();

    let mut rules = RulesEngine::new();
    info!(target: "init", "Reading rules: {:?}", code_dir);
    rules.load_dir(&code_dir).unwrap();
    let rules = Arc::new(rules);

    let mut problems = ProblemStorage::new();
    info!(target: "init", "Reading problems: {:?}", problems_dir);
    problems.load_dir(&problems_dir).unwrap();

    for p in problems.problems {
        println!("{} {}", "Problem".bold().green(), p);
        let mut solution = Solution::new(&p, rules.clone());
        match solution.solve() {
            Ok(_) => {
                println!("{} {}", "Solution:".italic().blue(), solution);
            }
            Err(e) => {
                println!("{} {} {}", "Solution:".italic().blue(), e.red(), solution);
            }
        };
    }
}

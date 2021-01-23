extern crate bigdecimal;
extern crate chrono;
extern crate clap;
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

mod statement;

mod core;
mod dump;
mod logger;
mod parser;
mod predefine;
mod problem;
mod rule;
mod settings;
mod solver;
mod utils;

use clap::{App, Arg};
use colored::*;
use core::{rule::RulesEngine, symbols::load_symbols};
use dump::{Dumper, FileDumper};
use logger::log_init;
use settings::Settings;
use solver::{problem::ProblemStorage, solution::Solution};
use std::{cell::RefCell, path::Path, rc::Rc, sync::Arc};

fn main() {
    let matches = App::new("Minerva")
        .version("1.0")
        .author("Quotique <just.std@gmail.com>")
        .about("Minerva core develop/debug enviroment")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .about("Sets a custom config file")
                .default_value("./config/local.json")
                .takes_value(true),
        )
        .arg(
            Arg::new("only")
                .short('o')
                .long("only")
                .value_name("ID")
                .about("Runs only spcified problem")
                .takes_value(true),
        )
        .arg(
            Arg::new("symbols")
                .short('s')
                .long("symbols")
                .value_name("DIR")
                .about("Specify symbols path")
                .takes_value(true),
        )
        .arg(
            Arg::new("problems")
                .short('p')
                .long("problems")
                .value_name("DIR")
                .about("Specify problems path")
                .takes_value(true),
        )
        .arg(
            Arg::new("dump")
                .short('d')
                .long("dump")
                .about("Dump solution trace into a file")
                .takes_value(false),
        )
        .get_matches();

    let settings = Settings::new(&matches.value_of("config").unwrap())
        .map_err(|e| {
            println!("Config error: {:?}", e);
            e
        })
        .unwrap_or_else(|_| {
            std::process::exit(-1);
        });
    log_init(&settings.logger);

    let code_dir = matches
        .value_of("symbols")
        .map(|x| x.to_owned())
        .or(settings.symbols_dir)
        .unwrap_or_else(|| {
            println!("Symbols dir is not specified");
            std::process::exit(-1);
        });
    let code_dir = Path::new(code_dir.as_str());

    let problems_dir = matches
        .value_of("problems")
        .map(|x| x.to_owned())
        .or(settings.problems_dir)
        .unwrap_or_else(|| {
            println!("Problems dir is not specified");
            std::process::exit(-1);
        });
    let problems_dir = Path::new(problems_dir.as_str());

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
        if let Some(only) = matches.value_of("only") {
            let id = format!("{:x}", p.id);
            if !id.starts_with(only) && !id.ends_with(only) {
                continue;
            }
        }
        println!("{} {}", "Problem".bold().green(), p);
        let mut solution = Solution::new(&p, rules.clone());
        if matches.is_present("dump") {
            let dumper = Rc::new(RefCell::new(Box::new(FileDumper::new(
                format!("dumps/{:x}.dump", p.id).as_str(),
            )) as Box<dyn Dumper>));
            solution.add_dumper(dumper);
        }
        match solution.solve() {
            Ok(_) => {
                println!("{} {}", "Solution:".italic().blue(), solution);
            }
            Err(e) => {
                println!(
                    "{} {} {}",
                    "Solution:".italic().blue(),
                    e.to_string().red(),
                    solution
                );
            }
        };
    }
}

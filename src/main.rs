#![allow(clippy::redundant_field_names)]
#![allow(clippy::module_inception)]

extern crate bigdecimal;
extern crate chrono;
extern crate clap;
extern crate colored;
extern crate config;
extern crate derive_builder;
extern crate derive_more;
extern crate eyre;
extern crate fern;
extern crate multi_map;
extern crate num;
extern crate num_bigint;
extern crate parking_lot;
extern crate peg;
extern crate serde;
extern crate smartstring;
extern crate trees;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod statement;

mod parser;
mod predefine;
mod problem;
mod rule;
mod utils;

use clap::{App, Arg};
use colored::*;
use std::sync::Arc;

use problem::Solution;
use utils::{log_init, DirectoryParser, Dumper, DumperConfig, Settings};

fn main() {
    let matches = App::new("Minerva")
        .version(env!("CARGO_PKG_VERSION"))
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

    let settings = Settings::new(matches.value_of("config").unwrap())
        .map_err(|e| {
            println!("Config error: {:?}", e);
            e
        })
        .unwrap_or_else(|_| {
            std::process::exit(-1);
        });
    log_init(&settings.logger);

    let parser = DirectoryParser::new(
        matches
            .value_of("symbols")
            .map(|x| x.to_owned())
            .or(settings.symbols_dir)
            .unwrap_or_else(|| {
                println!("Symbols dir is not specified");
                std::process::exit(-1);
            }),
        matches
            .value_of("problems")
            .map(|x| x.to_owned())
            .or(settings.problems_dir)
            .unwrap_or_else(|| {
                println!("Problems dir is not specified");
                std::process::exit(-1);
            }),
    );

    let rules_engine = Arc::new(parser.load_rules().unwrap());
    let problems = parser.load_problems().unwrap();

    for p in problems {
        if let Some(only) = matches.value_of("only") {
            let id = format!("{:x}", p.id);
            if !id.starts_with(only) && !id.ends_with(only) {
                continue;
            }
        }
        println!("{} {}", "Problem".bold().green(), p);
        let p_id = p.id;
        let mut solution = Solution::new(
            p,
            rules_engine.clone(),
            Dumper::new(DumperConfig {
                sink:     if matches.is_present("dump") {
                    "file".into()
                } else {
                    "none".into()
                },
                filename: format!("dumps/{:x}.dump", p_id),
            }),
        );

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

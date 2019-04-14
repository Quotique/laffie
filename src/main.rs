#[macro_use]
extern crate log;
extern crate chrono;
extern crate fern;

extern crate config;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod core;
mod parser;
mod settings;

use std::env;
use std::path::Path;
use std::str::FromStr;

use settings::Logger;
use settings::Settings;

use core::rules_engine::RulesEngine;
use core::symbols::all_symbols;

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
        .chain(std::io::stdout())
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

    if args.len() < 2 {
        println!("Usage: {} <code_dir>", &args[0]);
        return;
    }

    let dir = Path::new(&args[1][..]);

    info!(target: "init", "Reading symbols");
    all_symbols().load_dir(&dir).unwrap();
    let mut rules = RulesEngine::new();
    info!(target: "init", "Reading rules");
    rules.load_dir(&dir).unwrap();
}

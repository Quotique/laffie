#[macro_use]
extern crate log;
extern crate chrono;
extern crate fern;

extern crate config;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod settings;
mod parser;

use std::str::FromStr;

use settings::Logger;
use settings::Settings;

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
        }).level(log_level)
        .chain(std::io::stdout())
        .chain(fern::log_file(&config.filename).unwrap())
        .apply()
        .unwrap();

    info!(target: "log_init", "Log initialized with params: {:?}", config);
    info!(target: "log_init", "Current log level: {:?}", log_level);
}

fn main() {
    let settings = Settings::new();
    match settings {
        Ok(ref s) => log_init(&s.logger),
        Err(e) => println!("Config error: {:?}", e),
    }

    info!(target: "main", "Hello world");
}

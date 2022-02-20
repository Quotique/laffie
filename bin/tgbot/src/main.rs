mod commands;
mod settings;
mod text;

use std::{env, sync::Arc};

use clap::{Arg, Command};
use futures::Future;
use telebot::Bot;

use database::{ProblemDb, UserDb};
use mcore::{rule::RulesEngine, utils::log_init};
use parser::DirectoryParser;

use commands::{problem_handler, problems_list_handler, start_handler};
use settings::Settings;

fn run_bot(engine: Arc<RulesEngine>, problems_db: Arc<ProblemDb>, users_db: Arc<UserDb>) {
    let mut bot = Bot::new("5171464247:AAGR6y0SYZ8zGzx_vni6ITT7dVeirLvVKHE").update_interval(200);

    let problem = problem_handler(&mut bot, engine, problems_db.clone(), users_db.clone());
    let problems_list = problems_list_handler(&mut bot, problems_db, users_db);

    let start = start_handler(&mut bot);

    bot.run_with(problems_list.join(problem.join(start)));
}

fn main() {
    let matches = Command::new("LaffieBot")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Quotique <just.std@gmail.com>")
        .about("Telegram interface for Minerva Core")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .default_value("./config/local.json")
                .takes_value(true),
        )
        .arg(
            Arg::new("symbols")
                .short('s')
                .long("symbols")
                .value_name("DIR")
                .help("Specify symbols path")
                .takes_value(true),
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
        "".to_owned(),
    );

    let rules_engine = Arc::new(parser.load_rules().unwrap());
    let problems_db = Arc::new(ProblemDb::open(settings.problems_db).unwrap());
    let users_db = Arc::new(UserDb::open(settings.users_db).unwrap());

    run_bot(rules_engine, problems_db, users_db)
}

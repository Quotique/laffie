mod commands;
mod settings;
mod text;

use std::{env, sync::Arc};

use clap::{Arg, Command};
use futures::StreamExt;
use telegram_bot::*;

use database::{ProblemDb, UserDb};
use mcore::{rule::RulesEngine, utils::log_init};
use parser::DirectoryParser;

use commands::process_update;
use settings::Settings;

rust_i18n::i18n!("locales");

async fn run_bot(
    token: &str,
    engine: Arc<RulesEngine>,
    problems_db: Arc<ProblemDb>,
    users_db: Arc<UserDb>,
) {
    let api = Api::new(token);

    let mut stream = api.stream();

    while let Some(update) = stream.next().await {
        if let Ok(update) = update {
            process_update(
                update,
                &api,
                engine.clone(),
                problems_db.clone(),
                users_db.clone(),
            )
            .await;
        }
    }
}

#[tokio::main]
async fn main() {
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
        "".to_string(),
    );

    let rules_engine = Arc::new(parser.load_rules().unwrap());
    let problems_db = Arc::new(ProblemDb::open(settings.problems_db).unwrap());
    let users_db = Arc::new(UserDb::open(settings.users_db).unwrap());

    run_bot(&settings.api_token, rules_engine, problems_db, users_db).await
}

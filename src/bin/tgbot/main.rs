mod commands;
mod pagination;
mod settings;
mod text;

use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use futures::StreamExt;
use telegram_bot::*;

use database::{TaskDb, UserDb};
use mcore::rule::RulesEngine;
use parser::DirectoryParser;

use commands::process_update;
use settings::Settings;

rust_i18n::i18n!("locales");

/// Telegram interface
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    #[clap(short, long, default_value = "./config/local.json")]
    config: PathBuf,

    /// Specify symbols path
    #[clap(short, long)]
    symbols: Option<PathBuf>,
}

async fn run_bot(
    token: &str,
    engine: Arc<RulesEngine>,
    tasks_db: Arc<TaskDb>,
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
                tasks_db.clone(),
                users_db.clone(),
            )
            .await;
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

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
            .or(settings.symbols_dir.map(|x| x.into()))
            .unwrap_or_else(|| {
                println!("Symbols dir is not specified");
                std::process::exit(-1);
            }),
        "".into(),
    );

    let rules_engine = Arc::new(parser.load_rules().unwrap());
    let tasks_db = Arc::new(TaskDb::open(settings.tasks_db).unwrap());
    let users_db = Arc::new(UserDb::open(settings.users_db).unwrap());

    run_bot(&settings.api_token, rules_engine, tasks_db, users_db).await
}

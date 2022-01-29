mod static_data;

use std::{env, sync::Arc};

use clap::{App, Arg};
use futures::{stream::Stream, Future};
use telebot::{functions::*, Bot};

use mcore::{
    parser::{ra, ProblemParser},
    problem::Solution,
    rule::RulesEngine,
    utils::{log_init, DirectoryParser, Dumper, DumperConfig, Settings},
};

use static_data::Static;

fn solve(problem_text: String, engine: Arc<RulesEngine>) -> Result<String, String> {
    let states = ra::problem(&problem_text).map_err(|e| e.to_string())?;
    let problem = ProblemParser::with(&states)
        .parse()
        .map_err(|e| e.to_string())?;

    let mut solution = Solution::new(
        problem,
        engine,
        Dumper::new(DumperConfig {
            sink:     "none".into(),
            filename: "".to_owned(),
        }),
    );

    let result = match solution.solve() {
        Ok(_) => format!("{} {}", "Solution:", solution),
        Err(e) => format!("{} {} {}", "Solution:", e, solution),
    };
    let plain_bytes = strip_ansi_escapes::strip(result.as_bytes()).unwrap();
    Ok(std::str::from_utf8(&plain_bytes).unwrap().to_owned())
}

fn run_bot(engine: Arc<RulesEngine>) {
    let mut bot = Bot::new("5171464247:AAGR6y0SYZ8zGzx_vni6ITT7dVeirLvVKHE").update_interval(200);

    let problem = bot
        .new_cmd("/problem")
        .and_then(move |(bot, msg)| {
            let text = format!("problem {}", msg.text.unwrap().clone());

            match solve(text, engine.clone()) {
                Ok(s) => bot.message(msg.chat.id, s).send(),
                Err(s) => bot.message(msg.chat.id, s).send(),
            }
        })
        .for_each(|_| Ok(()));

    let start = bot
        .new_cmd("/start")
        .and_then(|(bot, msg)| {
            let text = Static::start();
            bot.message(msg.chat.id, text).send()
        })
        .for_each(|_| Ok(()));

    bot.run_with(problem.join(start));
}

fn main() {
    let matches = App::new("LaffieBot")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Quotique <just.std@gmail.com>")
        .about("Telegram interface for Minerva Core")
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
            Arg::new("symbols")
                .short('s')
                .long("symbols")
                .value_name("DIR")
                .about("Specify symbols path")
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
    run_bot(rules_engine)
}

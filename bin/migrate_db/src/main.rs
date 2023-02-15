mod settings;

use clap::{Arg, Command};

use database::{ProblemDb, ProblemRecord, UserDb, UserRecord};
use mcore::utils::log_init;

use settings::Settings;

fn update_problems(problems_db: ProblemDb) -> eyre::Result<()> {
    for p in problems_db.iter_old() {
        let problem = ProblemRecord::from(p);
        problems_db.put(&problem)?;
    }

    Ok(())
}

fn update_users(users_db: UserDb) -> eyre::Result<()> {
    for u in users_db.iter_old() {
        let user = UserRecord::from(u);
        users_db.put(&user)?;
    }

    Ok(())
}

fn main() {
    let matches = Command::new("MigrateDB")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Quotique <just.std@gmail.com>")
        .about("DB migration tool")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .default_value("./config/local.json")
                .takes_value(true),
        )
        .get_matches();

    let settings = Settings::new(matches.value_of("config").unwrap())
        .map_err(|e| {
            println!("Config error: {e:?}");
            e
        })
        .unwrap_or_else(|_| {
            std::process::exit(-1);
        });
    log_init(&settings.logger);

    let problems_db = ProblemDb::open(&settings.problems_db).unwrap();
    problems_db.backup(&settings.problems_backup).unwrap();
    if let Err(e) = update_problems(problems_db) {
        println!("error: {e}");
        ProblemDb::restore(&settings.problems_db, &settings.problems_backup).unwrap();
    }

    let users_db = UserDb::open(&settings.users_db).unwrap();
    users_db.backup(&settings.users_backup).unwrap();
    if let Err(e) = update_users(users_db) {
        println!("error: {e}");
        UserDb::restore(&settings.users_db, &settings.problems_db).unwrap();
    }
}

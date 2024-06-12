mod settings;

use clap::{Arg, Command};

use database::{TaskDb, TaskRecord, UserDb, UserRecord};
use mcore::utils::log_init;

use settings::Settings;

fn update_tasks(tasks_db: TaskDb) -> eyre::Result<()> {
    for p in tasks_db.iter_old() {
        let task = TaskRecord::from(p);
        tasks_db.put(&task)?;
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
    let _log_guard = log_init(&settings.logger);

    let tasks_db = TaskDb::open(&settings.tasks_db).unwrap();
    tasks_db.backup(&settings.tasks_backup).unwrap();
    if let Err(e) = update_tasks(tasks_db) {
        println!("error: {e}");
        TaskDb::restore(&settings.tasks_db, &settings.tasks_backup).unwrap();
    }

    let users_db = UserDb::open(&settings.users_db).unwrap();
    users_db.backup(&settings.users_backup).unwrap();
    if let Err(e) = update_users(users_db) {
        println!("error: {e}");
        UserDb::restore(&settings.users_db, &settings.tasks_db).unwrap();
    }
}

use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub filename: String,
    pub level:    String,
}

pub fn log_init(config: &Config) {
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
        //.chain(std::io::stdout())
        .chain(fern::log_file(&config.filename).unwrap())
        .apply()
        .unwrap();

    info!(target: "init", "Log initialized with params: {:?}", config);
    info!(target: "init", "Current log level: {:?}", log_level);
}

pub fn stdout_log_init(level: &str) {
    let log_level = log::LevelFilter::from_str(&level).unwrap_or(log::LevelFilter::Debug);
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
        .apply()
        .unwrap();

    info!(target: "init", "Log initialized with level: {}", level);
}

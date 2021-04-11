use std::collections::HashMap;
use std::str::FromStr;

use log::LevelFilter;
use serde::{de, Deserialize};

#[derive(Debug, Deserialize)]
pub struct Level(#[serde(deserialize_with = "deserialize_level")] LevelFilter);

#[derive(Debug, Deserialize)]
pub struct Config {
    pub filename:      String,
    pub level:         Level,
    pub target_levels: HashMap<String, Level>,
}

fn deserialize_level<'de, D>(deserializer: D) -> Result<LevelFilter, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = <String>::deserialize(deserializer)?;
    Ok(LevelFilter::from_str(s.as_str()).map_err(de::Error::custom)?)
}

pub fn log_init(config: &Config) {
    let mut logger = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Utc::now().format("[%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(config.level.0);
    for (target, level) in config.target_levels.iter() {
        logger = logger.level_for(target.clone(), level.0);
    }
    logger
        //.chain(std::io::stdout())
        .chain(fern::log_file(&config.filename).unwrap())
        .apply()
        .unwrap();

    info!(target: "init", "Log initialized with params: {:?}", config);
}

#[allow(dead_code)]
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

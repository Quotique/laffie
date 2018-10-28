use config::{Config, ConfigError, File};

#[derive(Debug, Deserialize)]
pub struct Logger {
    pub filename: String,
    pub level: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub logger: Logger,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name("config/local.json")).unwrap();

        s.try_into()
    }
}

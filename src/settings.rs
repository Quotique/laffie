use config::{Config, ConfigError, File};
use logger::Config as LogConfig;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub logger: LogConfig,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name("config/local.json")).unwrap();

        s.try_into()
    }
}

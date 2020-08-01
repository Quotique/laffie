use config::{Config, ConfigError, File};
use logger::Config as LogConfig;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub logger:       LogConfig,
    pub symbols_dir:  Option<String>,
    pub problems_dir: Option<String>,
}

impl Settings {
    pub fn new(file_name: &str) -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name(file_name)).unwrap();

        s.try_into()
    }
}

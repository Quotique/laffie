use std::path::PathBuf;

use config::{Config, ConfigError, File};
use serde_derive::Deserialize;

use mcore::utils::LogConfig;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub logger:          LogConfig,
    pub problems_db:     PathBuf,
    pub users_db:        PathBuf,
    pub problems_backup: PathBuf,
    pub users_backup:    PathBuf,
}

impl Settings {
    pub fn new(file_name: &str) -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name(file_name)).unwrap();

        s.try_into()
    }
}

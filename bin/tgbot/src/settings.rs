use std::path::PathBuf;

use config::{Config, ConfigError, File, FileFormat};
use serde_derive::Deserialize;

use mcore::utils::LogConfig;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub logger:      LogConfig,
    pub symbols_dir: Option<String>,
    pub problems_db: PathBuf,
    pub users_db:    PathBuf,
    pub api_token:   String,
}

impl Settings {
    pub fn new(file_name: &str) -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::new(file_name, FileFormat::Json))
            .build()?
            .try_deserialize()
    }
}

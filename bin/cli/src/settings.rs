use std::path::{Path, PathBuf};

use config::{Config, ConfigError, File};
use serde_derive::Deserialize;

use mcore::utils::LogConfig;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub logger:       LogConfig,
    pub symbols_dir:  Option<PathBuf>,
    pub problems_dir: Option<PathBuf>,
}

impl Settings {
    pub fn new<P: AsRef<Path>>(file: P) -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::from(file.as_ref())).unwrap();

        s.try_into()
    }
}

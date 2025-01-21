use std::path::Path;

use config::{Config, ConfigError, File, FileFormat};
use serde_derive::Deserialize;

use utils::LogConfig;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub logger: LogConfig,
    // pub symbols_dir: Option<PathBuf>,
    // pub tasks_dir:   Option<PathBuf>,
}

impl Settings {
    pub fn new<P: AsRef<Path>>(file: P) -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::new(file.as_ref().to_str().unwrap(), FileFormat::Yaml))
            .build()?
            .try_deserialize()
    }
}

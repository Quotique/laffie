use std::{io, path::PathBuf};

use clap::Parser;
use config::{Config, ConfigError, File, FileFormat};
use serde_derive::Deserialize;

use utils::LogConfig;

use crate::theme::ThemeName;

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(skip)]
    pub config_path:       PathBuf,
    pub logger:            LogConfig,
    pub symbols:           PathBuf,
    pub tasks:             PathBuf,
    pub exec_deadline:     usize,
    pub solve_parallelism: usize,
    #[serde(default)]
    pub theme:             ThemeName,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let args = Args::parse();
        let default_parallelism = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1) as i64;

        let mut settings: Settings = Config::builder()
            .set_default("symbols", "symbols")?
            .set_default("tasks", "tasks")?
            .set_default("exec_deadline", 100000)?
            .set_default("solve_parallelism", default_parallelism)?
            .set_default("theme", "dark")?
            .add_source(File::new(args.config.to_str().unwrap(), FileFormat::Yaml))
            .set_override_option(
                "symbols",
                args.symbols.map(|x| x.to_str().unwrap().to_owned()),
            )?
            .set_override_option("tasks", args.tasks.map(|x| x.to_str().unwrap().to_owned()))?
            .set_override_option("exec_deadline", args.exec_deadline)?
            .build()?
            .try_deserialize()?;
        settings.config_path = args.config;
        Ok(settings)
    }

    pub fn save(&self) -> io::Result<()> {
        let mut value: serde_yaml::Value = if self.config_path.exists() {
            let text = std::fs::read_to_string(&self.config_path)?;
            serde_yaml::from_str(&text).unwrap_or(serde_yaml::Value::Mapping(Default::default()))
        } else {
            serde_yaml::Value::Mapping(Default::default())
        };
        let map = value
            .as_mapping_mut()
            .ok_or_else(|| io::Error::other("config yaml root is not a mapping"))?;
        let theme = serde_yaml::to_value(self.theme).map_err(io::Error::other)?;
        map.insert("exec_deadline".into(), (self.exec_deadline as u64).into());
        map.insert(
            "solve_parallelism".into(),
            (self.solve_parallelism as u64).into(),
        );
        map.insert("theme".into(), theme);
        let text = serde_yaml::to_string(&value).map_err(io::Error::other)?;
        std::fs::write(&self.config_path, text)
    }
}

/// Core develop/debug environment
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    #[clap(short, long, default_value = "./config/tui.yaml")]
    config: PathBuf,

    /// Specify symbols path
    #[clap(short, long)]
    symbols: Option<PathBuf>,

    /// Specify tasks path
    #[clap(short = 'p', long)]
    tasks: Option<PathBuf>,

    /// Specify tasks DB path
    // #[clap(short = 'd', long)]
    // tasks_db: Option<PathBuf>,

    /// Execution deadline (in cycles) for individual problem
    #[clap(short, long)]
    exec_deadline: Option<i64>,
}

use std::path::PathBuf;

use clap::Parser;
use config::{Config, ConfigError, File, FileFormat};
use serde_derive::Deserialize;

use utils::LogConfig;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub logger:        LogConfig,
    pub symbols:       PathBuf,
    pub tasks:         PathBuf,
    pub exec_deadline: usize,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let args = Args::parse();

        Config::builder()
            .set_default("symbols", "symbols")?
            .set_default("tasks", "tasks")?
            .set_default("exec_deadline", 100000)?
            .add_source(File::new(args.config.to_str().unwrap(), FileFormat::Yaml))
            .set_override_option(
                "symbols",
                args.symbols.map(|x| x.to_str().unwrap().to_owned()),
            )?
            .set_override_option("tasks", args.tasks.map(|x| x.to_str().unwrap().to_owned()))?
            .set_override_option("exec_deadline", args.exec_deadline)?
            .build()?
            .try_deserialize()
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

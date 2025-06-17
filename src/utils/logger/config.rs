use std::{collections::HashMap, fs::create_dir_all, path::Path, str::FromStr};

use file_rotate::{compression::Compression, suffix::AppendCount, ContentLimit, FileRotate};
use serde::{de, Deserialize as _};
use serde_derive::Deserialize;
use slog::{o, Drain, Level, Record};
use slog_async::{Async, OverflowStrategy};

use super::{
    filter::Filter,
    format::{print_msg_header, system_time_write},
};

const DEFAULT_CHANNEL_SIZE: usize = 100 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct ConfLevel(#[serde(deserialize_with = "deserialize_level")] pub Level);

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_channel_size")]
    pub channel_size:      usize,
    pub filename:          String,
    pub level:             ConfLevel,
    pub num_files:         usize,
    pub file_rotate_bytes: usize,
    pub target_levels:     HashMap<String, ConfLevel>,
}

fn deserialize_level<'de, D>(deserializer: D) -> Result<Level, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = <String>::deserialize(deserializer)?;
    Level::from_str(s.as_str()).map_err(|_| {
        de::Error::invalid_value(
            de::Unexpected::Str(s.as_str()),
            &"[Error, Warn, Into, Debug, Trace]",
        )
    })
}

fn default_channel_size() -> usize {
    DEFAULT_CHANNEL_SIZE
}

impl Config {
    pub fn init(&self) -> slog_scope::GlobalLoggerGuard {
        let mut file = self.open_file().expect("unable to open log file");
        file.rotate().expect("unable to rotate log file");

        let f = Filter::new(
            self.level.0,
            self.target_levels.iter().map(|(x, y)| (x.clone(), y.0)),
        );

        let drain = slog_term::FullFormat::new(slog_term::PlainDecorator::new(file))
            .use_custom_header_print(print_msg_header)
            .use_custom_timestamp(system_time_write)
            .use_file_location()
            .build()
            .filter(Box::new(move |record: &Record| -> bool {
                f.filter(record.module(), record.level())
            }))
            .fuse();

        let guard = slog_scope::set_global_logger(slog::Logger::root(
            Async::new(drain)
                .chan_size(self.channel_size)
                .overflow_strategy(OverflowStrategy::Block)
                .build()
                .fuse(),
            o!(),
        ));
        slog_stdlog::init().unwrap();

        log::info!(target: "init", "Log initialized with params: {:?}", self);
        guard
    }

    fn open_file(&self) -> std::io::Result<FileRotate<AppendCount>> {
        let path = Path::new(&self.filename);
        if let Some(p) = path.parent() {
            create_dir_all(p)?;
        }
        Ok(FileRotate::new(
            path,
            AppendCount::new(self.num_files),
            ContentLimit::Bytes(self.file_rotate_bytes),
            Compression::None,
            None,
        ))
    }
}

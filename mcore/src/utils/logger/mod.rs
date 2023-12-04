mod config;
mod filter;
mod format;

use std::{fs::create_dir_all, path::Path};

use file_rotate::{compression::Compression, suffix::AppendCount, ContentLimit, FileRotate};
use slog::{o, Drain, Record};
use slog_async::Async;

use filter::Filter;
use format::{print_msg_header, system_time_write};

pub use config::Config;

pub fn log_init(config: &Config) -> slog_scope::GlobalLoggerGuard {
    let mut file = open_file(&config.filename, config.num_files, config.file_rotate_bytes)
        .expect("unable to open log file");
    file.rotate().expect("unable to rotate log file");

    let f = Filter::new(
        config.level.0,
        config.target_levels.iter().map(|(x, y)| (x.clone(), y.0)),
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
        Async::new(drain).chan_size(1024 * 1024).build().fuse(),
        o!(),
    ));
    slog_stdlog::init().unwrap();

    info!(target: "init", "Log initialized with params: {:?}", config);
    guard
}

fn open_file(
    output: &str,
    file_count: usize,
    bytes_limit: usize,
) -> std::io::Result<FileRotate<AppendCount>> {
    let path = Path::new(output);
    if let Some(p) = path.parent() {
        create_dir_all(p)?;
    }
    Ok(FileRotate::new(
        path,
        AppendCount::new(file_count),
        ContentLimit::Bytes(bytes_limit),
        Compression::None,
        None,
    ))
}

use std::{io, io::Write};

use chrono::Timelike;
use slog::Record;
use slog_term::{CountingWriter, RecordDecorator, ThreadSafeTimestampFn};

pub fn print_msg_header(
    fn_timestamp: &dyn ThreadSafeTimestampFn<Output = io::Result<()>>,
    mut rd: &mut dyn RecordDecorator,
    record: &Record,
    use_file_location: bool,
) -> io::Result<bool> {
    rd.start_timestamp()?;
    fn_timestamp(&mut rd)?;

    rd.start_whitespace()?;
    write!(rd, " ")?;

    rd.start_level()?;
    write!(rd, "{}", record.level().as_short_str())?;

    rd.start_whitespace()?;
    write!(rd, " ")?;

    if use_file_location {
        rd.start_location()?;
        write!(rd, "[{}]", record.module())?;

        rd.start_whitespace()?;
        write!(rd, " ")?;
    }

    rd.start_msg()?;
    let mut count_rd = CountingWriter::new(&mut rd);
    write!(count_rd, "{}", record.msg())?;
    Ok(count_rd.count() != 0)
}

pub fn system_time_write(out: &mut dyn std::io::Write) -> std::io::Result<()> {
    let now = chrono::Utc::now();
    write!(
        out,
        "[{:02}:{:02}:{:02}.{:03}]",
        now.hour(),
        now.minute(),
        now.second(),
        now.nanosecond() / 1_000_000
    )
}

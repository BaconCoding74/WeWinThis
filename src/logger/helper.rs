use std::io::Write;
use std::io::BufWriter;
use crate::config::{PRINT_INFO_TO_TERMINAL};
use crate::logging::default::LogLevel;

pub fn drain_queue<Type, Log, Timestamp, Level, LogFormat>(
    mut pop_one: Log,
    module: &str,
    timestamp_of: Timestamp,
    level_of: Level,
    format_record: LogFormat,
    writer: &mut BufWriter<std::fs::File>,
    drained_any: &mut bool,
) where
    Log: FnMut() -> Option<Type>,
    Timestamp: Fn(&Type) -> u32,
    Level: Fn(&Type) -> LogLevel,
    LogFormat: Fn(&Type) -> String,
{
    while let Some(record) = pop_one() {
        *drained_any = true;

        let ts = timestamp_of(&record);
        let level = level_of(&record);
        let msg = format_record(&record);

        let line = format!("[{:>8} ms][{:<6}][{:?}] {}", ts, module, level, msg);

        let _ = writeln!(writer, "{}", line);

        if should_print_to_terminal(level) {
            #[cfg(feature = "enable_terminal_colorization")]
            println!("{}", colorize(level, &line));

            #[cfg(not(feature = "enable_terminal_colorization"))]
            println!("{}", line);
        }
    }
}
fn should_print_to_terminal(level: LogLevel) -> bool {
    match level {
        LogLevel::Info => PRINT_INFO_TO_TERMINAL,
        LogLevel::Warn | LogLevel::Error | LogLevel::Critical => true,
    }
}

fn colorize(level: LogLevel, line: &str) -> String {
    match level {
        LogLevel::Info => line.to_string(),
        LogLevel::Warn => format!("\x1b[33m{}\x1b[0m", line),
        LogLevel::Error => format!("\x1b[31m{}\x1b[0m", line),
        LogLevel::Critical => format!("\x1b[1;31m{}\x1b[0m", line),
    }
}
use std::{
    fs::File,
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Logger {
    file: File,
}

impl Logger {
    pub fn new(file: File) -> Self {
        Self { file }
    }

    pub fn log(&mut self, msg: &str) {
        if let Err(e) = writeln!(self.file, "{}", msg) {
            eprintln!("Logging failed: {}", e);
        }
    }
}

pub fn log_command(
    logger: &mut Logger,
    timestamp: u128,
    cmd_type: &str,
    latency_ms: u128,
    status: &str,
) {
    logger.log(&format!(
        "{},{},{},{}",
        timestamp, cmd_type, latency_ms, status
    ));
}

pub fn log_telemetry(
    logger: &mut Logger,
    timestamp: u128,
    sequence: u32,
    temperature: f32,
    voltage: f32,
    latency_ms: u128,
    status: &str,
) {
    logger.log(&format!(
        "{},{},{},{},{},{}",
        timestamp, sequence, temperature, voltage, latency_ms, status
    ));
}

pub fn log_system_state(logger: &mut Logger, timestamp: u128, mode: &str, reason: &str) {
    logger.log(&format!("{},{},{}", timestamp, mode, reason));
}

pub fn log_jitter(
    logger: &mut Logger,
    timestamp: u128,
    task_name: &str,
    expected_interval_ms: u128,
    actual_interval_ms: u128,
    jitter_ms: u128,
) {
    logger.log(&format!(
        "{},{},{},{},{}",
        timestamp, task_name, expected_interval_ms, actual_interval_ms, jitter_ms
    ));
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

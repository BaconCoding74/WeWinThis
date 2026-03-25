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
    data_type: &str,
    value1: f32,
    value2: f32,
    value3: f32,
    latency_ms: u128,
) {
    logger.log(&format!(
        "{},{},{},{},{},{},{}",
        timestamp, sequence, data_type, value1, value2, value3, latency_ms
    ));
}

pub fn log_fault(
    logger: &mut Logger,
    timestamp: u128,
    sequence: u32,
    alert_code: &str,
    temperature: f32,
    action: &str,
) {
    logger.log(&format!(
        "{},{},{},{},{}",
        timestamp, sequence, alert_code, temperature, action
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

pub fn log_critical_alert(
    logger: &mut Logger,
    timestamp: u128,
    alert_type: &str,
    latency_ms: u128,
    description: &str,
) {
    logger.log(&format!(
        "{},{},{},{}",
        timestamp, alert_type, latency_ms, description
    ));
}

pub fn log_decode_latency(
    logger: &mut Logger,
    timestamp: u128,
    data_type: &str,
    decode_latency_ms: f32,
    threshold_ms: f32,
) {
    let status = if decode_latency_ms > threshold_ms {
        "EXCEEDS_THRESHOLD"
    } else {
        "OK"
    };
    logger.log(&format!(
        "{},{},{:.3},{},{:.3}",
        timestamp, data_type, decode_latency_ms, status, threshold_ms
    ));
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

use crate::logging::default::LogLevel;

#[derive(Debug, Copy, Clone)]
pub enum HealthLogCode {
    Heartbeat,
    DownlinkQueueFillPct,
    EnterDegraded,
    ExitDegraded,
    GyroSampleDropped,
    BatterySampleDropped,
}

#[derive(Debug, Clone, Copy)]
pub struct HealthLogRecord {
    pub level: LogLevel,
    pub timestamp_ms: u32,
    pub code: HealthLogCode,
    pub value: i32,
}
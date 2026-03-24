#[derive(Debug, Copy, Clone)]
pub enum HealthLogCode {
    Heartbeat,
    DownlinkQueueFillPct,
    EnterDegraded,
    ExitDegraded,
}

#[derive(Debug, Copy, Clone)]
pub struct HealthLogRecord {
    pub timestamp_ms: u32,
    pub code: HealthLogCode,
    pub value: i32,
}
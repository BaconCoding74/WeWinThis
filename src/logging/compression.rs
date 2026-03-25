use crate::logging::default::LogLevel;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLogCode {
    DownlinkReserveBlocked = 6000,
    DownlinkPushFailed = 6001,
    RunComplete = 6002,
}

#[derive(Debug, Clone, Copy)]
pub struct CompressionLogRecord {
    pub level: LogLevel,
    pub timestamp_ms: u32,
    pub code: CompressionLogCode,
    pub value: i32,
}
use crate::logging::default::LogLevel;
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLogRejectReason {
    None = 0,
    UnsafeState = 1,
    InvalidPayload = 2,
    InterlockActive = 3,
    UnknownCommand = 4,
    ResponseQueueFull = 5,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLogCode {
    CommandReceived = 7000,
    ModeSetNormal = 7001,
    ModeSetDegraded = 7002,
    PingHandled = 7003,
    CommandExecuted = 7004,
    CommandRejected = 7005,
    ResponseQueued = 7006,
    ResponseDropped = 7007,
}

#[derive(Debug, Clone, Copy)]
pub struct CommandLogRecord {
    pub level: LogLevel,
    pub timestamp_ms: u32,
    pub code: CommandLogCode,
    pub cmd_seq: u32,
    pub cmd_type: u8,
    pub reject_reason: CommandLogRejectReason,
    pub value: i32,
}
use crate::logging::default::LogLevel;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntennaLogCode {
    AlignmentDisabled = 4000,
    TargetReached = 4001,
    StepApplied = 4002,
    ReadyChanged = 4003,
}

#[derive(Debug, Clone, Copy)]
pub struct AntennaLogRecord {
    pub level: LogLevel,
    pub timestamp_ms: u32,
    pub code: AntennaLogCode,
    pub target_deg: i16,
    pub current_deg: i16,
    pub error_deg: i16,
    pub applied_step_deg: i16,
    pub ready: bool,
}
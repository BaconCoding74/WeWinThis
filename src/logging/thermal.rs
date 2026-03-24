use crate::logging::default::LogLevel;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalLogCode {
    Recovery = 1,
    OverheatDetected = 2,
    SensorReadMiss = 3,
    SensorFaultEscalated = 4,
    DeadlineMiss = 5,
    StatusDropped = 6,
    AlertDropped = 7,
}

#[derive(Debug, Clone, Copy)]
pub struct ThermalLogRecord {
    pub level: LogLevel,
    pub timestamp_ms: u32,
    pub code: ThermalLogCode,
    pub value: i32,
}
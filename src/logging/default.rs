
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info = 0,
    Warn = 1,
    Error = 2,
    Critical = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSource {
    Thermal = 1,
    Scheduler = 2,
    Communication = 3,
    CommandExec = 4,
    Battery = 5,
    Gyro = 6,
    Antenna = 7,
    Health = 8,
}

#[derive(Debug, Clone, Copy)]
pub struct LogRecord {
    pub source: LogSource,
    pub level: LogLevel,
    pub timestamp_ms: u32,
    pub code: u16,
    pub value: i32,
}
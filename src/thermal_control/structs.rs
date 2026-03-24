use crate::config::{DEFAULT_MAX_TEMP_X10, DEFAULT_TARGET_TEMP_X10};

#[derive(Debug, Clone, Copy)]
pub struct ThermalSample {
    pub seq: u64,
    pub timestamp_ms: u32,
    pub temp_c_x10: i16,
    pub flags: u8,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalAlertCode {
    Overheat = 1,
    SensorFault = 2,
    EmergencyShutdown = 3,
    Recovery = 4,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalActionCode {
    None = 0,
    IncreaseCooling = 1,
    ForceMaxCooling = 2,
    ReduceCooling = 3,
    EmergencyShutdown = 4,
    ResumeNormal = 5,
}

#[derive(Debug, Clone, Copy)]
pub struct ThermalStatusMsg {
    pub timestamp_ms: u32,
    pub temp_x10: i16,
    pub target_temp_x10: i16,
    pub actuator_pct: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct ThermalAlertMsg {
    pub timestamp_ms: u32,
    pub alert_code: ThermalAlertCode,
    pub temp_x10: i16,
    pub flags: u8,
    pub action_code: ThermalActionCode,
}

#[derive(Debug, Clone, Copy)]
pub enum ThermalToComm {
    Status(ThermalStatusMsg),
    Alert(ThermalAlertMsg),
}

#[derive(Debug, Clone, Copy)]
pub struct ThermalState {
    pub last_temp_x10: i16,
    pub actuator_pct: u8,
    pub missed_cycles: u8,
    pub safety_alert: bool,
    pub overheat: bool,
}

impl ThermalState {
    pub const fn new() -> Self {
        Self {
            last_temp_x10: 0,
            actuator_pct: 0,
            missed_cycles: 0,
            safety_alert: false,
            overheat: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThermalConfig {
    pub target_temp_x10: i16,
    pub max_temp_x10: i16,
}

impl ThermalConfig {
    pub const fn new() -> Self {
        Self {
            target_temp_x10: DEFAULT_TARGET_TEMP_X10,
            max_temp_x10: DEFAULT_MAX_TEMP_X10,
        }
    }
}
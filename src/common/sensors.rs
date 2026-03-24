use std::time::Instant;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum SensorType {
    Thermal = 0,
    Gyro = 1,
    Battery = 2,
}

#[derive(Debug, Clone, Copy)]
pub struct GyroMsg {
    pub timestamp_ms: u32,
    pub x_mdps: i16,
    pub y_mdps: i16,
    pub z_mdps: i16,
}

#[derive(Debug, Clone, Copy)]
pub struct BatteryMsg {
    pub timestamp_ms: u32,
    pub mv: u16,
    pub ma: i16,
    pub pct: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum SensorData {
    Gyro(GyroMsg),
    Battery(BatteryMsg),
}

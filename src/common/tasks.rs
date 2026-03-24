
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskId {
    Thermal,
    Gyro,
    Battery,
    Health,
    Antenna,
    CommandExec,
    Compression,
}
#[derive(Debug, Copy, Clone)]
pub struct AntennaLogRecord {
    pub timestamp_ms: u32,
    pub target_deg: i16,
    pub current_deg: i16,
    pub error_deg: i16,
    pub applied_step_deg: i16,
    pub ready: bool,
}
use std::sync::atomic::{AtomicBool, AtomicI16, AtomicU8, AtomicU32, Ordering};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    Safe = 0,
    Emergency = 1,
}

pub struct SystemState {
    pub _stop: AtomicBool,
    pub mode: AtomicU8,
    pub _visibility_open: AtomicBool,

    pub last_temp_x10: AtomicI16,
    pub thermal_overheat: AtomicBool,
    pub thermal_alert: AtomicBool,
    pub fault_detect_time: AtomicU32,

    pub _last_sequence: AtomicU32,
}

impl SystemState {
    pub fn new() -> Self {
        Self {
            _stop: AtomicBool::new(false),
            mode: AtomicU8::new(RuntimeMode::Safe as u8),
            _visibility_open: AtomicBool::new(false),

            last_temp_x10: AtomicI16::new(0),
            thermal_overheat: AtomicBool::new(false),
            thermal_alert: AtomicBool::new(false),
            fault_detect_time: AtomicU32::new(0),

            _last_sequence: AtomicU32::new(0),
        }
    }

    pub fn get_mode(&self) -> RuntimeMode {
        match self.mode.load(Ordering::Acquire) {
            1 => RuntimeMode::Emergency,
            _ => RuntimeMode::Safe,
        }
    }

    pub fn set_mode(&self, mode: RuntimeMode) {
        self.mode.store(mode as u8, Ordering::Release);
    }

    pub fn set_temperature(&self, temp_x10: i16) {
        self.last_temp_x10.store(temp_x10, Ordering::Release);
    }

    pub fn get_temperature(&self) -> f32 {
        self.last_temp_x10.load(Ordering::Acquire) as f32 / 10.0
    }

    pub fn is_overheated(&self) -> bool {
        self.thermal_overheat.load(Ordering::Acquire)
    }

    pub fn set_sequence(&self, seq: u32) {
        self._last_sequence.store(seq, Ordering::Release);
    }

    pub fn _get_sequence(&self) -> u32 {
        self._last_sequence.load(Ordering::Acquire)
    }

    pub fn _set_visibility(&self, visible: bool) {
        self._visibility_open.store(visible, Ordering::Release);
    }

    pub fn _is_visible(&self) -> bool {
        self._visibility_open.load(Ordering::Acquire)
    }

    pub fn set_fault_detect_time(&self, timestamp_ms: u32) {
        self.fault_detect_time
            .store(timestamp_ms, Ordering::Release);
    }

    pub fn get_fault_detect_time(&self) -> u32 {
        self.fault_detect_time.load(Ordering::Acquire)
    }

    pub fn has_active_fault(&self) -> bool {
        self.thermal_alert.load(Ordering::Acquire) || self.thermal_overheat.load(Ordering::Acquire)
    }

    pub fn _clear_fault(&self) {
        self.thermal_alert.store(false, Ordering::Release);
        self.thermal_overheat.store(false, Ordering::Release);
        self.fault_detect_time.store(0, Ordering::Release);
    }
}

use std::sync::atomic::{AtomicBool, AtomicI16, AtomicU8, Ordering};

#[derive(Debug)]
pub struct SystemState {
    pub stop: AtomicBool,
    pub mode: AtomicU8,
    pub visibility_open: AtomicBool,

    pub antenna_align_enabled: AtomicBool,
    pub antenna_ready: AtomicBool,
    pub antenna_target_deg: AtomicI16,
    pub antenna_current_deg: AtomicI16,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemMode {
    Normal = 0,
    Degraded = 1,
}

impl SystemMode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => SystemMode::Degraded,
            _ => SystemMode::Normal,
        }
    }
}

impl SystemState {
    pub fn new() -> Self {
        Self {
            stop: AtomicBool::new(false),
            mode: AtomicU8::new(SystemMode::Normal as u8),
            visibility_open: AtomicBool::new(false),

            antenna_align_enabled: AtomicBool::new(true),
            antenna_ready: AtomicBool::new(false),
            antenna_target_deg: AtomicI16::new(364),
            antenna_current_deg: AtomicI16::new(0),

        }
    }

    pub fn set_mode(&self, mode: SystemMode) {
        self.mode.store(mode as u8, Ordering::Release)
    }

    pub fn get_mode(&self) -> SystemMode {
        SystemMode::from_u8(self.mode.load(Ordering::Acquire))
    }

    pub fn set_visibility_open(&self, visibility_open: bool) {
        self.visibility_open.store(visibility_open, Ordering::Release)
    }
}
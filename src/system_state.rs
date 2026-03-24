#[derive(Debug, Clone, Copy)]
pub enum SystemMode {
    Safe,
    Emergency,
}

#[derive(Debug, Clone)]
pub struct SystemState {
    pub last_sequence: u32,
    pub system_mode: SystemMode,
    pub last_temperature: f32,
}

impl SystemState {
    pub fn new() -> Self {
        Self {
            last_sequence: 0,
            system_mode: SystemMode::Safe,
            last_temperature: 0.0,
        }
    }

    pub fn set_sequence(&mut self, seq: u32) {
        self.last_sequence = seq;
    }

    pub fn set_mode(&mut self, mode: SystemMode) {
        self.system_mode = mode;
    }

    pub fn set_temperature(&mut self, temp: f32) {
        self.last_temperature = temp;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SystemStateMessage {
    pub last_sequence: u32,
    pub system_mode: SystemMode,
    pub last_temperature: f32,
}

pub fn encode_system_state(
    last_sequence: u32,
    system_mode: SystemMode,
    last_temperature: f32,
) -> [u8; 14] {
    let mut payload = [0u8; 14];
    payload[0..4].copy_from_slice(&last_sequence.to_be_bytes());
    payload[4] = match system_mode {
        SystemMode::Safe => 1,
        SystemMode::Emergency => 2,
    };
    payload[5..9].copy_from_slice(&last_temperature.to_be_bytes());
    payload
}

pub fn decode_system_state(payload: &[u8; 14]) -> Result<SystemStateMessage, &'static str> {
    let last_sequence = u32::from_be_bytes(
        payload[0..4]
            .try_into()
            .map_err(|_| "Invalid system state sequence bytes")?,
    );

    let system_mode = match payload[4] {
        1 => SystemMode::Safe,
        2 => SystemMode::Emergency,
        _ => return Err("Invalid system mode"),
    };

    let last_temperature = f32::from_be_bytes(
        payload[5..9]
            .try_into()
            .map_err(|_| "Invalid temperature bytes")?,
    );

    Ok(SystemStateMessage {
        last_sequence,
        system_mode,
        last_temperature,
    })
}

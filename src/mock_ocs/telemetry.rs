use rand::Rng;

pub const TELEMETRY_SIZE: usize = 14;

#[derive(Debug, Clone)]
pub struct Telemetry {
    pub timestamp_ms: u64,
    pub temperature: i16,
    pub battery_mv: u16,
    pub antenna_angle: i16,
}

impl Telemetry {
    pub fn to_bytes(&self) -> [u8; TELEMETRY_SIZE] {
        let mut bytes = [0u8; TELEMETRY_SIZE];
        bytes[0..8].copy_from_slice(&self.timestamp_ms.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.temperature.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.battery_mv.to_le_bytes());
        bytes[12..14].copy_from_slice(&self.antenna_angle.to_le_bytes());
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < TELEMETRY_SIZE {
            return None;
        }

        Some(Self {
            timestamp_ms: u64::from_le_bytes(data[0..8].try_into().ok()?),
            temperature: i16::from_le_bytes(data[8..10].try_into().ok()?),
            battery_mv: u16::from_le_bytes(data[10..12].try_into().ok()?),
            antenna_angle: i16::from_le_bytes(data[12..14].try_into().ok()?),
        })
    }
}

pub struct TelemetryGenerator {
    pub rng: rand::rngs::ThreadRng,
    pub base_temperature: i16,
    pub base_battery: u16,
}

impl TelemetryGenerator {
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
            base_temperature: 20,
            base_battery: 8000,
        }
    }

    pub fn generate_normal(&mut self, timestamp_ms: u64) -> Telemetry {
        let temp_variation: i16 = self.rng.gen_range(-10..=10);
        let battery_drain: u16 = self.rng.gen_range(1..=5);
        let antenna_variation: i16 = self.rng.gen_range(-5..=5);

        Telemetry {
            timestamp_ms,
            temperature: self.base_temperature + temp_variation,
            battery_mv: self.base_battery.saturating_sub(battery_drain),
            antenna_angle: antenna_variation,
        }
    }

    pub fn generate_edge_case(&mut self, timestamp_ms: u64, case_type: u8) -> Telemetry {
        match case_type % 6 {
            0 => Telemetry {
                timestamp_ms,
                temperature: -50,
                battery_mv: self.base_battery,
                antenna_angle: 0,
            },
            1 => Telemetry {
                timestamp_ms,
                temperature: 125,
                battery_mv: self.base_battery,
                antenna_angle: 0,
            },
            2 => Telemetry {
                timestamp_ms,
                temperature: self.base_temperature,
                battery_mv: 2000,
                antenna_angle: 0,
            },
            3 => Telemetry {
                timestamp_ms,
                temperature: self.base_temperature,
                battery_mv: 0,
                antenna_angle: 0,
            },
            4 => Telemetry {
                timestamp_ms,
                temperature: self.base_temperature,
                battery_mv: self.base_battery,
                antenna_angle: -90,
            },
            _ => Telemetry {
                timestamp_ms,
                temperature: self.base_temperature,
                battery_mv: self.base_battery,
                antenna_angle: 90,
            },
        }
    }
}

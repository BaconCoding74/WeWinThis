use crate::thermal::{decode_status, ThermalStatusMsg, ThermalToComm};

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
    pub ma: i16,
    pub mv: i16,
    pub pct: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum TelemetryData {
    Gyro(GyroMsg),
    Battery(BatteryMsg),
    Thermal(ThermalStatusMsg),
}

pub fn decode_telemetry(payload: &[u8]) -> Result<TelemetryData, &'static str> {
    if payload.is_empty() {
        return Err("empty telemetry payload");
    }

    match payload[0] {
        // 🔹 Gyro
        1 => {
            if payload.len() != 11 {
                return Err("invalid gyro payload");
            }

            let timestamp = u32::from_le_bytes(payload[1..5].try_into().unwrap());
            let x = i16::from_le_bytes(payload[5..7].try_into().unwrap());
            let y = i16::from_le_bytes(payload[7..9].try_into().unwrap());
            let z = i16::from_le_bytes(payload[9..11].try_into().unwrap());

            Ok(TelemetryData::Gyro(GyroMsg {
                timestamp_ms: timestamp,
                x_mdps: x,
                y_mdps: y,
                z_mdps: z,
            }))
        }

        // 🔹 Battery
        2 => {
            if payload.len() != 10 {
                return Err("invalid battery payload");
            }

            let timestamp = u32::from_le_bytes(payload[1..5].try_into().unwrap());
            let ma = i16::from_le_bytes(payload[5..7].try_into().unwrap());
            let mv = i16::from_le_bytes(payload[7..9].try_into().unwrap());
            let pct = payload[9];

            Ok(TelemetryData::Battery(BatteryMsg {
                timestamp_ms: timestamp,
                ma,
                mv,
                pct,
            }))
        }

        // 🔹 Thermal Status
        3 => match decode_status(payload)? {
            ThermalToComm::Status(s) => Ok(TelemetryData::Thermal(s)),
            _ => Err("unexpected thermal variant"),
        },

        _ => Err("unknown telemetry subtype"),
    }
}

use std::sync::atomic::Ordering;

use crate::{
    logger::now_ms,
    system_state::{RuntimeMode, SystemState},
};

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
    pub _timestamp_ms: u32,
    pub temp_x10: i16,
    pub target_temp_x10: i16,
    pub _actuator_pct: u8,
    pub _flags: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct ThermalAlertMsg {
    pub _timestamp_ms: u32,
    pub alert_code: ThermalAlertCode,
    pub temp_x10: i16,
    pub _flags: u8,
    pub action_code: ThermalActionCode,
}

#[derive(Debug, Clone, Copy)]
pub enum ThermalToComm {
    Status(ThermalStatusMsg),
    Alert(ThermalAlertMsg),
}

pub fn decode_status(payload: &[u8]) -> Result<ThermalToComm, &'static str> {
    if payload.len() < 11 || payload[0] != 3 {
        return Err("invalid thermal status payload");
    }

    let timestamp_ms =
        u32::from_le_bytes(payload[1..5].try_into().map_err(|_| "invalid timestamp")?);

    let temp_x10 = i16::from_le_bytes(payload[5..7].try_into().map_err(|_| "invalid temp")?);

    let target_temp_x10 = i16::from_le_bytes(
        payload[7..9]
            .try_into()
            .map_err(|_| "invalid target temp")?,
    );

    let actuator_pct = payload[9];
    let flags = payload[10];

    Ok(ThermalToComm::Status(ThermalStatusMsg {
        _timestamp_ms: timestamp_ms,
        temp_x10,
        target_temp_x10,
        _actuator_pct: actuator_pct,
        _flags: flags,
    }))
}

pub fn decode_alert(payload: &[u8]) -> Result<ThermalToComm, &'static str> {
    if payload.len() != 10 || payload[0] != 1 {
        return Err("invalid thermal alert payload");
    }

    let timestamp_ms =
        u32::from_le_bytes(payload[1..5].try_into().map_err(|_| "invalid timestamp")?);

    let alert_code = match payload[5] {
        1 => ThermalAlertCode::Overheat,
        2 => ThermalAlertCode::SensorFault,
        3 => ThermalAlertCode::EmergencyShutdown,
        4 => ThermalAlertCode::Recovery,
        _ => return Err("invalid alert code"),
    };

    let temp_x10 = i16::from_le_bytes(payload[6..8].try_into().map_err(|_| "invalid temp")?);

    let flags = payload[8];

    let action_code = match payload[9] {
        0 => ThermalActionCode::None,
        1 => ThermalActionCode::IncreaseCooling,
        2 => ThermalActionCode::ForceMaxCooling,
        3 => ThermalActionCode::ReduceCooling,
        4 => ThermalActionCode::EmergencyShutdown,
        5 => ThermalActionCode::ResumeNormal,
        _ => return Err("invalid action code"),
    };

    Ok(ThermalToComm::Alert(ThermalAlertMsg {
        _timestamp_ms: timestamp_ms,
        alert_code,
        temp_x10,
        _flags: flags,
        action_code,
    }))
}

pub fn update_system_state_from_thermal(state: &SystemState, msg: ThermalToComm, seq: u32) {
    state.set_sequence(seq);

    match msg {
        ThermalToComm::Status(s) => {
            state.set_temperature(s.temp_x10);

            // Clear overheat if temperature back to safe
            if s.temp_x10 < 800 {
                state.thermal_overheat.store(false, Ordering::Release);
            }
        }

        ThermalToComm::Alert(a) => {
            state.set_temperature(a.temp_x10);

            match a.alert_code {
                ThermalAlertCode::Overheat => {
                    state.thermal_overheat.store(true, Ordering::Release);
                    state.thermal_alert.store(true, Ordering::Release);
                    state.set_fault_detect_time(now_ms() as u32);
                }

                ThermalAlertCode::EmergencyShutdown => {
                    state.set_mode(RuntimeMode::Emergency);
                    state.set_fault_detect_time(now_ms() as u32);
                }

                ThermalAlertCode::Recovery => {
                    state.thermal_overheat.store(false, Ordering::Release);
                    state.thermal_alert.store(false, Ordering::Release);
                    state.set_fault_detect_time(0);
                }

                _ => {}
            }
        }
    }
}

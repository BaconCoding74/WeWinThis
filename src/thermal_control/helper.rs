use crate::common::system_state::SystemMode;
use crate::config::{ThermalLogSPSCBuffer, ThermalSPSCBuffer};
use crate::logging::default::LogLevel;
use crate::logging::thermal::{ThermalLogCode, ThermalLogRecord};
use crate::thermal_control::structs::{ThermalActionCode, ThermalAlertCode, ThermalAlertMsg, ThermalState, ThermalToComm};

pub fn read_thermal_sensor(seq: u64) -> Option<i16> {
    let result = seq % 40;

    if result == 0 {
        return None;
    }
    else if result < 10 {
        Some(350)
    }
    else if result < 37 {
        Some(600)
    }
    else {
        Some(750)
    }
}

pub fn build_thermal_flags(valid: bool, overheat: bool, safety_alert: bool) -> u8 {
    let mut flags = 0u8;
    if valid { flags |= 1 << 0; }
    if overheat { flags |= 1 << 1; }
    if safety_alert { flags |= 1 << 2; }
    flags
}

pub fn compute_actuator_pct(
    temp_x10: i16,
    setpoint_x10: i16,
    high_limit_x10: i16,
    mode: SystemMode,
) -> u8 {

    if temp_x10 >= high_limit_x10 {
        return 100;
    }

    if temp_x10 <= setpoint_x10 {
        return 0;
    }

    let delta = (temp_x10 - setpoint_x10) as i32;

    let pct = match mode {
        SystemMode::Normal => (delta / 2).clamp(0, 100),
        SystemMode::Degraded => (delta).clamp(0, 100),
    };

    pct as u8
}

#[inline]
fn handle_thermal_miss(
    thermal_state: &mut ThermalState,
    thermal_log_q: &ThermalLogSPSCBuffer,
    thermal_to_comm_q: &ThermalSPSCBuffer,
    timestamp_ms: u32,
    miss_code: ThermalLogCode,
) {
    thermal_state.missed_cycles = thermal_state.missed_cycles.saturating_add(1);

    let _ = thermal_log_q.push(ThermalLogRecord {
        level: LogLevel::Warn,
        timestamp_ms,
        code: miss_code,
        value: thermal_state.missed_cycles as i32,
    });

    if thermal_state.missed_cycles > 3 {
        thermal_state.safety_alert = true;
        thermal_state.actuator_pct = 100;

        let alert = ThermalAlertMsg {
            timestamp_ms,
            alert_code: ThermalAlertCode::SensorFault,
            temp_x10: thermal_state.last_temp_x10,
            flags: build_thermal_flags(false, false, true),
            action_code: ThermalActionCode::ForceMaxCooling,
        };

        let _ = thermal_to_comm_q.push(ThermalToComm::Alert(alert));

        let _ = thermal_log_q.push(ThermalLogRecord {
            level: LogLevel::Error,
            timestamp_ms,
            code: ThermalLogCode::SensorFaultEscalated,
            value: thermal_state.missed_cycles as i32,
        });
    }
}

#[inline]
fn handle_thermal_recovery(
    thermal_state: &mut ThermalState,
    thermal_log_q: &ThermalLogSPSCBuffer,
    timestamp_ms: u32,
) {
    if thermal_state.missed_cycles > 0 {
        let _ = thermal_log_q.push(ThermalLogRecord {
            level: LogLevel::Info,
            timestamp_ms,
            code: ThermalLogCode::Recovery,
            value: thermal_state.missed_cycles as i32,
        });
    }

    thermal_state.missed_cycles = 0;
}
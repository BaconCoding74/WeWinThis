use crate::common::system_state::SystemMode;
use crate::config::{ThermalLogSPSCBuffer, ThermalSPSCBuffer, THERMAL_MISS_LIMIT, THERMAL_RECOVERY_LIMIT_MS};
use crate::logging::default::LogLevel;
use crate::logging::thermal::{ThermalLogCode, ThermalLogRecord};
use crate::thermal_control::structs::{ThermalActionCode, ThermalAlertCode, ThermalAlertMsg, ThermalState, ThermalStats, ThermalStatusMsg, ThermalToComm};

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
pub fn try_log_thermal(
    thermal_log_q: &ThermalLogSPSCBuffer,
    thermal_stats: &mut ThermalStats,
    record: ThermalLogRecord,
) {
    if thermal_log_q.push(record).is_err() {
        thermal_stats.dropped_logs = thermal_stats.dropped_logs.saturating_add(1);

        if matches!(record.level, LogLevel::Error | LogLevel::Critical) {
            thermal_stats.dropped_critical_logs = thermal_stats.dropped_critical_logs.saturating_add(1);
        }
    }
}

#[inline]
pub fn try_send_thermal_alert(
    thermal_to_comm_q: &ThermalSPSCBuffer,
    thermal_log_q: &ThermalLogSPSCBuffer,
    thermal_stats: &mut ThermalStats,
    alert: ThermalAlertMsg,
    drop_level: LogLevel,
) {
    if thermal_to_comm_q.push(ThermalToComm::Alert(alert)).is_err() {
        thermal_stats.dropped_alerts = thermal_stats.dropped_alerts.saturating_add(1);

        try_log_thermal(
            thermal_log_q,
            thermal_stats,
            ThermalLogRecord {
                level: drop_level,
                timestamp_ms: alert.timestamp_ms,
                code: ThermalLogCode::AlertDropped,
                value: alert.temp_x10 as i32,
            },
        );
    }
}

#[inline]
pub fn try_send_thermal_status(
    thermal_to_comm_q: &ThermalSPSCBuffer,
    thermal_log_q: &ThermalLogSPSCBuffer,
    thermal_stats: &mut ThermalStats,
    status: ThermalStatusMsg,
) {
    if thermal_to_comm_q.push(ThermalToComm::Status(status)).is_err() {
        try_log_thermal(
            thermal_log_q,
            thermal_stats,
            ThermalLogRecord {
                level: LogLevel::Warn,
                timestamp_ms: status.timestamp_ms,
                code: ThermalLogCode::StatusDropped,
                value: status.temp_x10 as i32,
            },
        );
    }
}

#[inline]
pub fn handle_thermal_miss(
    thermal_state: &mut ThermalState,
    thermal_log_q: &ThermalLogSPSCBuffer,
    thermal_to_comm_q: &ThermalSPSCBuffer,
    thermal_stats: &mut ThermalStats,
    timestamp_ms: u32,
) {
    thermal_state.missed_cycles = thermal_state.missed_cycles.saturating_add(1);
    thermal_stats.dropped_samples = thermal_stats.dropped_samples.saturating_add(1);

    try_log_thermal(
        thermal_log_q,
        thermal_stats,
        ThermalLogRecord {
            level: LogLevel::Warn,
            timestamp_ms,
            code: ThermalLogCode::SensorReadMiss,
            value: thermal_state.missed_cycles as i32,
        },
    );

    if thermal_state.missed_cycles > THERMAL_MISS_LIMIT && !thermal_state.sensor_fault_active {
        thermal_state.sensor_fault_active = true;
        thermal_state.recovery_started_ms = timestamp_ms;
        thermal_state.safety_alert = true;
        thermal_state.actuator_pct = 100;

        thermal_stats.alerts_raised = thermal_stats.alerts_raised.saturating_add(1);

        try_send_thermal_alert(
            thermal_to_comm_q,
            thermal_log_q,
            thermal_stats,
            ThermalAlertMsg {
                timestamp_ms,
                alert_code: ThermalAlertCode::SensorFault,
                temp_x10: thermal_state.last_temp_x10,
                flags: build_thermal_flags(false, false, true),
                action_code: ThermalActionCode::ForceMaxCooling,
            },
            LogLevel::Error,
        );

        try_log_thermal(
            thermal_log_q,
            thermal_stats,
            ThermalLogRecord {
                level: LogLevel::Error,
                timestamp_ms,
                code: ThermalLogCode::SensorFaultEscalated,
                value: thermal_state.missed_cycles as i32,
            },
        );

        try_log_thermal(
            thermal_log_q,
            thermal_stats,
            ThermalLogRecord {
                level: LogLevel::Info,
                timestamp_ms,
                code: ThermalLogCode::RecoveryStarted,
                value: 0,
            },
        );
    }
}

#[inline]
pub fn handle_thermal_recovery(
    thermal_state: &mut ThermalState,
    thermal_log_q: &ThermalLogSPSCBuffer,
    thermal_to_comm_q: &ThermalSPSCBuffer,
    thermal_stats: &mut ThermalStats,
    timestamp_ms: u32,
    temp_x10: i16,
) -> bool {
    if thermal_state.sensor_fault_active {
        let recovery_time_ms = timestamp_ms.saturating_sub(thermal_state.recovery_started_ms);

        if recovery_time_ms > THERMAL_RECOVERY_LIMIT_MS {
            try_log_thermal(
                thermal_log_q,
                thermal_stats,
                ThermalLogRecord {
                    level: LogLevel::Critical,
                    timestamp_ms,
                    code: ThermalLogCode::RecoveryTimeout,
                    value: recovery_time_ms as i32,
                },
            );

            try_send_thermal_alert(
                thermal_to_comm_q,
                thermal_log_q,
                thermal_stats,
                ThermalAlertMsg {
                    timestamp_ms,
                    alert_code: ThermalAlertCode::EmergencyShutdown,
                    temp_x10,
                    flags: build_thermal_flags(true, thermal_state.overheat, true),
                    action_code: ThermalActionCode::MissionAbort,
                },
                LogLevel::Critical,
            );

            try_log_thermal(
                thermal_log_q,
                thermal_stats,
                ThermalLogRecord {
                    level: LogLevel::Critical,
                    timestamp_ms,
                    code: ThermalLogCode::MissionAborted,
                    value: recovery_time_ms as i32,
                },
            );

            return false;
        }

        try_send_thermal_alert(
            thermal_to_comm_q,
            thermal_log_q,
            thermal_stats,
            ThermalAlertMsg {
                timestamp_ms,
                alert_code: ThermalAlertCode::Recovery,
                temp_x10,
                flags: build_thermal_flags(true, thermal_state.overheat, false),
                action_code: ThermalActionCode::ResumeNormal,
            },
            LogLevel::Error,
        );

        try_log_thermal(
            thermal_log_q,
            thermal_stats,
            ThermalLogRecord {
                level: LogLevel::Info,
                timestamp_ms,
                code: ThermalLogCode::RecoveryCompleted,
                value: recovery_time_ms as i32,
            },
        );

        thermal_state.sensor_fault_active = false;
    }

    true
}
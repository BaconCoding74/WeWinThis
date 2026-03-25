use std::sync::atomic::Ordering;
use crate::config::{ThermalLogSPSCBuffer, THERMAL_FAULT_PERIOD_MS};
use crate::logging::default::LogLevel;
use crate::logging::thermal::{ThermalLogCode, ThermalLogRecord};
use crate::thermal_control::helper::{read_thermal_sensor, try_log_thermal};
use crate::thermal_control::structs::ThermalStats;

#[derive(Debug, Clone, Copy)]
pub struct ThermalFaultInjector {
    pub active: bool,
    pub fault_started_ms: u32,
    pub fault_duration_ms: u32,
    pub next_fault_at_ms: u32,
    pub fault_count: u32,
}

impl ThermalFaultInjector {
    pub const fn new() -> Self {
        Self {
            active: false,
            fault_started_ms: 0,
            fault_duration_ms: 0,
            next_fault_at_ms: THERMAL_FAULT_PERIOD_MS,
            fault_count: 0,
        }
    }
}

#[inline]
pub fn read_thermal_sensor_with_fault(
    seq: u64,
    timestamp_ms: u32,
    injector: &mut ThermalFaultInjector,
    thermal_log_q: &ThermalLogSPSCBuffer,
    thermal_stats: &mut ThermalStats,
) -> Option<i16> {
    if !injector.active && timestamp_ms >= injector.next_fault_at_ms {
        injector.active = true;
        injector.fault_started_ms = timestamp_ms;
        injector.fault_count += 1;
        injector.next_fault_at_ms = injector.next_fault_at_ms.saturating_add(THERMAL_FAULT_PERIOD_MS);

        injector.fault_duration_ms = match injector.fault_count {
            1 => 80,
            2 => 150,
            3 => 300,
            _ => 80,
        };

        thermal_stats.injected_faults = thermal_stats.injected_faults.saturating_add(1);

        try_log_thermal(
            thermal_log_q,
            thermal_stats,
            ThermalLogRecord {
                level: LogLevel::Warn,
                timestamp_ms,
                code: ThermalLogCode::FaultInjected,
                value: injector.fault_duration_ms as i32,
            },
        );
    }

    if injector.active {
        let elapsed = timestamp_ms.saturating_sub(injector.fault_started_ms);
        if elapsed < injector.fault_duration_ms {
            return None;
        }
        injector.active = false;
    }

    read_thermal_sensor(seq)
}
use std::sync::Arc;
use std::sync::atomic::{Ordering};
use std::thread;
use std::time::{Duration, Instant};
use crate::common::cpu_stats::{print_cpu_stats, CpuStats};
use crate::common::metrics::{print_task_report};
use crate::common::system_state::SystemState;
use crate::common::tasks::TaskId;
use crate::common::timing::{task_timing, TaskRuntime, TaskTiming};
use crate::config::{ThermalLogSPSCBuffer, ThermalSPSCBuffer};
use crate::logging::default::{LogLevel};
use crate::logging::thermal::{ThermalLogCode, ThermalLogRecord};
use crate::reports::thermal::ThermalReport;
use crate::thermal_control::fault_injection::{read_thermal_sensor_with_fault, ThermalFaultInjector};
use crate::thermal_control::helper::{build_thermal_flags, compute_actuator_pct, handle_thermal_miss, handle_thermal_recovery, read_thermal_sensor, try_log_thermal, try_send_thermal_alert, try_send_thermal_status};
use crate::thermal_control::structs::{ThermalActionCode, ThermalAlertCode, ThermalAlertMsg, ThermalConfig, ThermalState, ThermalStats, ThermalStatusMsg};

pub fn run_thermal_loop(
    thermal_to_comm_q: &ThermalSPSCBuffer,
    thermal_log_q: &ThermalLogSPSCBuffer,
    system_state: Arc<SystemState>,
    thermal_cfg: ThermalConfig,
) -> ThermalReport {

    let cpu_stats = CpuStats::new();
    let mut thermal_stats = ThermalStats::default();
    let mut thermal_state = ThermalState::new();
    let mut injector = ThermalFaultInjector::new();

    let start_time = Instant::now();
    let timing: TaskTiming = task_timing(TaskId::Thermal);
    let mut runtime = TaskRuntime::new(start_time);

    while !system_state.stop.load(Ordering::Acquire) {
        let now = Instant::now();

        while now >= runtime.next_release {
            let active_start = Instant::now();

            let released_at = Instant::now();
            let expected_at = runtime.next_release;
            let deadline_at = expected_at + timing.deadline;
            runtime.stats.record_release(expected_at, released_at);

            let task_start = Instant::now();
            let timestamp_ms = start_time.elapsed().as_millis() as u32;


            #[cfg(feature = "fault_injection")]
            let reading = read_thermal_sensor_with_fault(
                runtime.seq,
                timestamp_ms,
                &mut injector,
                thermal_log_q,
                &mut thermal_stats,
            );

            #[cfg(not(feature = "fault_injection"))]
            let reading = read_thermal_sensor(runtime.seq);

            match reading {
                Some(temp_x10) => {
                    thermal_state.last_temp_x10 = temp_x10;
                    thermal_state.overheat = temp_x10 >= thermal_cfg.max_temp_x10;

                    if thermal_state.sensor_fault_active {
                        let recovered_ok = handle_thermal_recovery(
                            &mut thermal_state,
                            thermal_log_q,
                            thermal_to_comm_q,
                            &mut thermal_stats,
                            timestamp_ms,
                            temp_x10,
                        );

                        if !recovered_ok {
                            system_state.stop.store(true, Ordering::Release);
                            break;
                        }
                    } else if thermal_state.missed_cycles > 0 {
                        thermal_state.missed_cycles = 0;

                        if !thermal_state.overheat {
                            thermal_state.safety_alert = false;
                        }
                    }

                    thermal_state.actuator_pct = if thermal_state.sensor_fault_active {
                        100
                    } else {
                        compute_actuator_pct(
                            temp_x10,
                            thermal_cfg.target_temp_x10,
                            thermal_cfg.max_temp_x10,
                            system_state.get_mode(),
                        )
                    };

                    try_send_thermal_status(
                        thermal_to_comm_q,
                        thermal_log_q,
                        &mut thermal_stats,
                        ThermalStatusMsg {
                            timestamp_ms,
                            temp_x10,
                            target_temp_x10: thermal_cfg.target_temp_x10,
                            actuator_pct: thermal_state.actuator_pct,
                            flags: build_thermal_flags(
                                true,
                                thermal_state.overheat,
                                thermal_state.safety_alert,
                            )},
                    );

                    let should_send_overheat =
                        thermal_state.overheat ||
                            thermal_state.sensor_fault_active ||
                            runtime.seq % 4 == 0;

                    if should_send_overheat {
                        thermal_state.safety_alert = true;

                        try_send_thermal_alert(
                            thermal_to_comm_q,
                            thermal_log_q,
                            &mut thermal_stats,
                            ThermalAlertMsg {
                                timestamp_ms,
                                alert_code: ThermalAlertCode::Overheat,
                                temp_x10,
                                flags: build_thermal_flags(true, true, true),
                                action_code: ThermalActionCode::IncreaseCooling,
                            },
                            LogLevel::Critical,
                        );

                        try_log_thermal(
                            thermal_log_q,
                            &mut thermal_stats,
                            ThermalLogRecord {
                                level: LogLevel::Critical,
                                timestamp_ms,
                                code: ThermalLogCode::OverheatDetected,
                                value: temp_x10 as i32,
                            },
                        );
                    }
                }

                None => {
                    handle_thermal_miss(
                        &mut thermal_state,
                        thermal_log_q,
                        thermal_to_comm_q,
                        &mut thermal_stats,
                        timestamp_ms,
                    );
                }
            }

            let finish = Instant::now();
            let missed = finish > deadline_at;

            if missed {
                try_log_thermal(
                    thermal_log_q,
                    &mut thermal_stats,
                    ThermalLogRecord {
                        level: LogLevel::Critical,
                        timestamp_ms: start_time.elapsed().as_millis() as u32,
                        code: ThermalLogCode::DeadlineMiss,
                        value: thermal_state.missed_cycles as i32,
                    },
                );
            }

            runtime.stats.record_completion(task_start, finish, missed);

            runtime.seq += 1;
            runtime.next_release += timing.period;
            cpu_stats.add_active(active_start.elapsed());
        }

        let idle_time = Instant::now();

        let wake_time = runtime.next_release - Duration::from_micros(100);
        thread::sleep(wake_time.saturating_duration_since(Instant::now()));
        cpu_stats.add_idle(idle_time.elapsed());
    }
    
    ThermalReport {
        runtime,
        cpu_stats,
        timing,
    }
}
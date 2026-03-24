use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};
use crate::common::metrics::{duration_div, print_task_report};
use crate::common::system_state::SystemState;
use crate::common::tasks::TaskId;
use crate::common::timing::{task_timing, TaskRuntime, TaskTiming};
use crate::config::{ThermalLogSPSCBuffer, ThermalSPSCBuffer};
use crate::logging::default::{LogLevel};
use crate::logging::thermal::{ThermalLogCode, ThermalLogRecord};
use crate::thermal_control::helper::{build_thermal_flags, compute_actuator_pct, read_thermal_sensor};
use crate::thermal_control::structs::{ThermalActionCode, ThermalAlertCode, ThermalAlertMsg, ThermalConfig, ThermalState, ThermalStatusMsg, ThermalToComm};

pub fn run_thermal_loop(
    thermal_to_comm_q: &ThermalSPSCBuffer,
    thermal_log_q: &ThermalLogSPSCBuffer,
    system_state: Arc<SystemState>,
    thermal_cfg: ThermalConfig,
) {
    let mut total = Duration::ZERO;
    let mut max_loop_time = Duration::ZERO;
    let mut tick: u64 = 0;

    let mut thermal_state = ThermalState {
        last_temp_x10: 0,
        actuator_pct: 0,
        missed_cycles: 0,
        safety_alert: false,
        overheat: false,
    };

    let start_time = Instant::now();
    let timing: TaskTiming = task_timing(TaskId::Thermal);
    let mut runtime = TaskRuntime::new(start_time);

    while !system_state.stop.load(Ordering::Acquire) {
        let now = Instant::now();

        while now >= runtime.next_release {
            let released_at = Instant::now();
            let expected_at = runtime.next_release;
            let deadline_at = expected_at + timing.deadline;
            runtime.stats.record_release(expected_at, released_at);

            let task_start = Instant::now();

            match read_thermal_sensor(runtime.seq) {
                Some(temp_x10) => {
                    if thermal_state.missed_cycles > 0 {
                        let _ = thermal_log_q.push(ThermalLogRecord {
                            level: LogLevel::Info,
                            timestamp_ms: start_time.elapsed().as_millis() as u32,
                            code: ThermalLogCode::Recovery,
                            value: 1,
                        });
                    }

                    thermal_state.last_temp_x10 = temp_x10;
                    thermal_state.missed_cycles = 0;
                    thermal_state.overheat = temp_x10 >= thermal_cfg.max_temp_x10;

                    thermal_state.actuator_pct = compute_actuator_pct(
                        temp_x10,
                        thermal_cfg.target_temp_x10,
                        thermal_cfg.max_temp_x10,
                        system_state.get_mode(),
                    );

                    let status = ThermalStatusMsg {
                        timestamp_ms: start_time.elapsed().as_millis() as u32,
                        temp_x10,
                        target_temp_x10: thermal_cfg.target_temp_x10,
                        actuator_pct: thermal_state.actuator_pct,
                        flags: build_thermal_flags(
                            true,
                            thermal_state.overheat,
                            thermal_state.safety_alert,
                        ),
                    };

                    if thermal_to_comm_q.push(ThermalToComm::Status(status)).is_err() {

                    }

                    if thermal_state.overheat {
                        thermal_state.safety_alert = true;

                        let alert = ThermalAlertMsg {
                            timestamp_ms: start_time.elapsed().as_millis() as u32,
                            alert_code: ThermalAlertCode::Overheat,
                            temp_x10,
                            flags: build_thermal_flags(true, true, true),
                            action_code: ThermalActionCode::IncreaseCooling,
                        };

                        let _ = thermal_to_comm_q.push(ThermalToComm::Alert(alert));

                        let _ = thermal_log_q.push(ThermalLogRecord {
                            level: LogLevel::Critical,
                            timestamp_ms: start_time.elapsed().as_millis() as u32,
                            code: ThermalLogCode::OverheatDetected,
                            value: temp_x10 as i32,
                        });
                    }
                }

                None => {
                    thermal_state.missed_cycles = thermal_state.missed_cycles.saturating_add(1);

                    let _ = thermal_log_q.push(ThermalLogRecord {
                        level: LogLevel::Warn,
                        timestamp_ms: start_time.elapsed().as_millis() as u32,
                        code: ThermalLogCode::SensorReadMiss,
                        value: thermal_state.missed_cycles as i32,
                    });

                    if thermal_state.missed_cycles > 3 {
                        thermal_state.safety_alert = true;
                        thermal_state.actuator_pct = 100;

                        let alert = ThermalAlertMsg {
                            timestamp_ms: start_time.elapsed().as_millis() as u32,
                            alert_code: ThermalAlertCode::SensorFault,
                            temp_x10: thermal_state.last_temp_x10,
                            flags: build_thermal_flags(false, false, true),
                            action_code: ThermalActionCode::ForceMaxCooling,
                        };

                        let _ = thermal_to_comm_q.push(ThermalToComm::Alert(alert));

                        let _ = thermal_log_q.push(ThermalLogRecord {
                            level: LogLevel::Error,
                            timestamp_ms: start_time.elapsed().as_millis() as u32,
                            code: ThermalLogCode::SensorFaultEscalated,
                            value: thermal_state.missed_cycles as i32,
                        });
                    }
                }
            }

            let finish = Instant::now();
            let missed = finish > deadline_at;

            if missed {
                let _ = thermal_log_q.push(ThermalLogRecord {
                    level: LogLevel::Critical,
                    timestamp_ms: start_time.elapsed().as_millis() as u32,
                    code: ThermalLogCode::DeadlineMiss,
                    value: thermal_state.missed_cycles as i32,
                });
            }

            runtime.stats.record_completion(task_start, finish, missed);

            runtime.seq += 1;
            runtime.next_release += timing.period;
        }

        let duration = now.elapsed();
        total += duration;
        max_loop_time = max_loop_time.max(duration);
        tick += 1;

        let wake_time = runtime.next_release - Duration::from_micros(100);
        thread::sleep(wake_time.saturating_duration_since(Instant::now()));
    }

    println!("================ Thermal Thread Report ================");
    println!("thermal runs   : {}", runtime.seq);

    if tick > 0 {
        println!("avg loop time  : {:?}", duration_div(total, tick));
    } else {
        println!("avg loop time  : 0ns");
    }

    println!("max loop time  : {:?}", max_loop_time);
    println!();

    print_task_report("Thermal", &timing, &runtime.stats, true);
    println!("=======================================================");
}
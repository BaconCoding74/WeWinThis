use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};
use crate::common::command_stats::{print_command_report, CommandStats};
use crate::common::job::Job;
use crate::common::metrics::{duration_div, elapsed_ms_u32, print_task_report};
use crate::common::system_state::{SystemState};
use crate::common::tasks::TaskId;
use crate::common::timing::{task_timing, TaskRuntime, TaskTiming};
use crate::config::{AntLogSPSCBuffer, DownlinkSPSCBuffer, HealthLogSPSCBuffer, SchedulerLogSPSCBuffer, SensorSPSCBuffer, ThermalSPSCBuffer, UplinkSPSCBuffer, READY_QUEUE_CAP};
use crate::logging::default::{LogLevel, LogRecord, LogSource};
use crate::queues::fixed_priority_queue::FixedPriorityQueue;
use crate::tasks::antenna::run_antenna_alignment_job;
use crate::tasks::command_exec::run_command_exec_job;
use crate::tasks::compression::run_compression_job;
use crate::tasks::health::run_health_monitor_job;
use crate::tasks::sensors::run_sensor_job;

fn release_periodic_job(
    ready_q: &mut FixedPriorityQueue<READY_QUEUE_CAP>,
    log_q: &SchedulerLogSPSCBuffer,
    task_id: TaskId,
    timing: &TaskTiming,
    runtime: &mut TaskRuntime,
    job_seq: &mut u64,
    now: Instant,
    tick: u64,
) {

    while now >= runtime.next_release {
        let released_at = Instant::now();
        let expected_at = runtime.next_release;
        let deadline_at = expected_at + timing.deadline;
        runtime.stats.record_release(expected_at, deadline_at);

        if let Err(_job) = ready_q.push(Job {
            id: task_id,
            priority: timing.priority,
            release_tick: tick,
            seq: *job_seq,
            released_at,
            expected_at,
            deadline_at,
        }) {
            let _ = log_q.push(LogRecord {
                source: LogSource::Scheduler,
                level: LogLevel::Critical,
                timestamp_ms: elapsed_ms_u32(now),
                code: 1003,
                value: 0,
            });
        };

        *job_seq += 1;
        runtime.next_release += timing.period;
    }
}

pub fn run_scheduler_loop(
    sensor_q: &SensorSPSCBuffer,
    thermal_q: &ThermalSPSCBuffer,
    downlink_q: &DownlinkSPSCBuffer,
    uplink_q: &UplinkSPSCBuffer,
    scheduler_log_q: &SchedulerLogSPSCBuffer,
    ant_log_q: &AntLogSPSCBuffer,
    health_log_q: &HealthLogSPSCBuffer,
    system_state: Arc<SystemState>
) {
    let mut ready_queue = FixedPriorityQueue::<READY_QUEUE_CAP>::new();
    let start_time = Instant::now();
    let mut total = Duration::ZERO;
    let mut max_loop_time = Duration::ZERO;
    let mut tick: u64 = 0;
    let mut job_seq: u64 = 0;
    let mut tx_seq: u64 = 0;

    let mut command_stats = CommandStats::new();

    let gyro_timing = task_timing(TaskId::Gyro);
    let battery_timing = task_timing(TaskId::Battery);
    let compression_timing = task_timing(TaskId::Compression);
    let health_timing = task_timing(TaskId::Health);
    let antenna_timing = task_timing(TaskId::Antenna);
    let cmd_exec_timing = task_timing(TaskId::CommandExec);

    let mut gyro_runtime = TaskRuntime::new(start_time);
    let mut battery_runtime = TaskRuntime::new(start_time);
    let mut compression_runtime = TaskRuntime::new(start_time);
    let mut health_runtime = TaskRuntime::new(start_time);
    let mut antenna_runtime = TaskRuntime::new(start_time);
    let mut cmd_exec_runtime = TaskRuntime::new(start_time);

    while !system_state.stop.load(Ordering::Acquire) {
        let now = Instant::now();

        release_periodic_job(
            &mut ready_queue,
            scheduler_log_q,
            TaskId::Gyro,
            &gyro_timing,
            &mut gyro_runtime,
            &mut job_seq,
            now,
            tick,
        );

        release_periodic_job(
            &mut ready_queue,
            scheduler_log_q,
            TaskId::Battery,
            &battery_timing,
            &mut battery_runtime,
            &mut job_seq,
            now,
            tick,
        );

        release_periodic_job(
            &mut ready_queue,
            scheduler_log_q,
            TaskId::Compression,
            &compression_timing,
            &mut compression_runtime,
            &mut job_seq,
            now,
            tick,
        );

        release_periodic_job(
            &mut ready_queue,
            scheduler_log_q,
            TaskId::Health,
            &health_timing,
            &mut health_runtime,
            &mut job_seq,
            now,
            tick,
        );

        release_periodic_job(
            &mut ready_queue,
            scheduler_log_q,
            TaskId::Antenna,
            &antenna_timing,
            &mut antenna_runtime,
            &mut job_seq,
            now,
            tick,
        );

        release_periodic_job(
            &mut ready_queue,
            scheduler_log_q,
            TaskId::CommandExec,
            &cmd_exec_timing,
            &mut cmd_exec_runtime,
            &mut job_seq,
            now,
            tick,
        );

        if let Some(job) = ready_queue.pop() {
            let task_start = Instant::now();

            match job.id {
                TaskId::CommandExec => {
                    run_command_exec_job(
                        uplink_q,
                        downlink_q,
                        &system_state,
                        scheduler_log_q,
                        &mut command_stats,
                        &mut tx_seq,
                    );

                    let finish = Instant::now();
                    let missed = finish > job.deadline_at;
                    cmd_exec_runtime.stats.record_completion(task_start, finish, missed);

                    if missed {
                        let _ = scheduler_log_q.push(LogRecord {
                            source: LogSource::Scheduler,
                            level: LogLevel::Critical,
                            timestamp_ms: elapsed_ms_u32(now),
                            code: 1003,
                            value: 0,
                        });
                    }
                }
                TaskId::Compression => {
                    run_compression_job(
                        sensor_q,
                        thermal_q,
                        downlink_q,
                        &mut tx_seq,
                    );

                    let finish = Instant::now();
                    let missed = finish > job.deadline_at;
                    compression_runtime.stats.record_completion(task_start, finish, missed);

                    if missed {
                        let _ = scheduler_log_q.push(LogRecord {
                            source: LogSource::Scheduler,
                            level: LogLevel::Critical,
                            timestamp_ms: elapsed_ms_u32(now),
                            code: 1006,
                            value: 0,
                        });
                    }
                }
                TaskId::Antenna => {
                    run_antenna_alignment_job(
                        task_start,
                        &system_state,
                        ant_log_q,
                    );

                    let finish = Instant::now();
                    let missed = finish > job.deadline_at;
                    antenna_runtime.stats.record_completion(task_start, finish, missed);

                    if missed {
                        let _ = scheduler_log_q.push(LogRecord {
                            source: LogSource::Scheduler,
                            level: LogLevel::Critical,
                            timestamp_ms: elapsed_ms_u32(now),
                            code: 1006,
                            value: 0,
                        });
                    }
                }
                TaskId::Health => {
                    run_health_monitor_job(
                        start_time,
                        &system_state,
                        downlink_q,
                        health_log_q,
                    );

                    let finish = Instant::now();
                    let missed = finish > job.deadline_at;
                    health_runtime.stats.record_completion(task_start, finish, missed);

                    if missed {
                        let _ = scheduler_log_q.push(LogRecord {
                            source: LogSource::Scheduler,
                            level: LogLevel::Critical,
                            timestamp_ms: elapsed_ms_u32(now),
                            code: 1006,
                            value: 0,
                        });
                    }
                }
                TaskId::Gyro => {
                    run_sensor_job(sensor_q, TaskId::Gyro, &mut gyro_runtime.seq);

                    let finish = Instant::now();
                    let missed = finish > job.deadline_at;
                    gyro_runtime.stats.record_completion(task_start, finish, missed);

                    if missed {
                        let _ = scheduler_log_q.push(LogRecord {
                            source: LogSource::Scheduler,
                            level: LogLevel::Critical,
                            timestamp_ms: elapsed_ms_u32(now),
                            code: 1003,
                            value: 0,
                        });
                    }
                }
                TaskId::Battery => {
                    run_sensor_job(sensor_q, TaskId::Battery, &mut battery_runtime.seq);

                    let finish = Instant::now();
                    let missed = finish > job.deadline_at;
                    battery_runtime.stats.record_completion(task_start, finish, missed);

                    if missed {
                        let _ = scheduler_log_q.push(LogRecord {
                            source: LogSource::Scheduler,
                            level: LogLevel::Critical,
                            timestamp_ms: elapsed_ms_u32(now),
                            code: 1004,
                            value: 0,
                        });
                    }
                }
                _ => {}
            }
        }
        else {
            let next_release = gyro_runtime.next_release
                .min(battery_runtime.next_release)
                .min(compression_runtime.next_release)
                .min(health_runtime.next_release)
                .min(antenna_runtime.next_release)
                .min(cmd_exec_runtime.next_release);

            let wake_time = next_release - Duration::from_micros(100);
            thread::sleep(wake_time.saturating_duration_since(Instant::now()));
        }

        let duration = now.elapsed();
        total += duration;
        max_loop_time = max_loop_time.max(duration);
        tick += 1;
    }

    println!("================ Scheduler Report ================");
    println!("gyro runs      : {}", gyro_runtime.seq);
    println!("battery runs   : {}", battery_runtime.seq);

    if tick > 0 {
        println!(
            "avg loop time  : {:?}",
            duration_div(total, tick)
        );
    } else {
        println!("avg loop time  : 0ns");
    }

    println!("max loop time  : {:?}", max_loop_time);
    println!();

    print_task_report("Gyro", &gyro_timing, &gyro_runtime.stats, true);
    println!();
    print_task_report("Battery", &battery_timing, &battery_runtime.stats, true);
    println!();
    print_task_report("Antenna", &antenna_timing, &antenna_runtime.stats, true);
    println!();
    print_task_report("Health", &health_timing, &health_runtime.stats, true);
    println!();
    print_command_report("Command", &command_stats);
    println!("==================================================");
}
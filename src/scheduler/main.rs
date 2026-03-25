use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};
use crate::common::command_stats::{CommandStats};
use crate::common::cpu_stats::CpuStats;
use crate::common::job::Job;
use crate::common::metrics::{TaskStats};
use crate::common::scheduler_stats::SchedulerStats;
use crate::common::system_state::{SystemState};
use crate::common::tasks::TaskId;
use crate::common::timing::{task_timing, TaskRuntime, TaskTiming};
use crate::config::{AntLogSPSCBuffer, CommandLogSPSCBuffer, CompressionLogSPSCBuffer, DownlinkSPSCBuffer, HealthLogSPSCBuffer, SchedulerLogSPSCBuffer, SensorSPSCBuffer, ThermalSPSCBuffer, UplinkSPSCBuffer, READY_QUEUE_CAP};
use crate::logging::default::{LogLevel};
use crate::logging::scheduler::SchedulerLogCode;
use crate::queues::fixed_priority_queue::FixedPriorityQueue;
use crate::reports::scheduler::SchedulerReport;
use crate::scheduler::helper::log_scheduler;
use crate::tasks::antenna::run_antenna_alignment_job;
use crate::tasks::command_exec::run_command_exec_job;
use crate::tasks::compression::run_compression_job;
use crate::tasks::health::run_health_monitor_job;
use crate::tasks::sensors::run_sensor_job;

fn release_periodic_job(
    q_stats: &mut SchedulerStats,
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
            seq: *job_seq,
            released_at,
            expected_at,
            deadline_at,
            release_tick: tick,
        }) {
            q_stats.job_dropped += 1;

            log_scheduler(
                q_stats,
                log_q,
                now,
                LogLevel::Error,
                SchedulerLogCode::ReadyQueueFull,
                task_id,
                0,
            );
        };

        *job_seq += 1;
        runtime.next_release += timing.period;
    }
}

fn record_task_completion(
    job_id: TaskId,
    stats: &mut TaskStats,
    q_stats: &mut SchedulerStats,
    log_q: &SchedulerLogSPSCBuffer,
    deadline: Instant,
    task_start: Instant,
) {
    let finish = Instant::now();
    let missed = finish > deadline;
    stats.record_completion(task_start, finish, missed);

    if missed {
        log_scheduler(
            q_stats,
            &log_q,
            finish,
            LogLevel::Critical,
            SchedulerLogCode::CompletionDeadlineMiss,
            job_id,
            0,
        );
    }
}

pub fn run_scheduler_loop(
    sensor_q: &SensorSPSCBuffer,
    thermal_q: &ThermalSPSCBuffer,
    downlink_q: &DownlinkSPSCBuffer,
    uplink_q: &UplinkSPSCBuffer,
    scheduler_log_q: &SchedulerLogSPSCBuffer,
    command_log_q: &CommandLogSPSCBuffer,
    ant_log_q: &AntLogSPSCBuffer,
    health_log_q: &HealthLogSPSCBuffer,
    compression_log_q: &CompressionLogSPSCBuffer,
    system_state: Arc<SystemState>
) -> SchedulerReport {
    let mut ready_queue = FixedPriorityQueue::<READY_QUEUE_CAP>::new();
    let start_time = Instant::now();
    let mut tick: u64 = 0;
    let mut job_seq: u64 = 0;
    let mut tx_seq: u64 = 0;

    let cpu_stats = CpuStats::new();
    let mut command_stats = CommandStats::new();
    let mut scheduler_q_stats = SchedulerStats::default();

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
            &mut scheduler_q_stats,
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
            &mut scheduler_q_stats,
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
            &mut scheduler_q_stats,
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
            &mut scheduler_q_stats,
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
            &mut scheduler_q_stats,
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
            &mut scheduler_q_stats,
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
                        command_log_q,
                        &mut command_stats,
                        &mut tx_seq,
                    );

                    record_task_completion(
                        job.id,
                        &mut cmd_exec_runtime.stats,
                        &mut scheduler_q_stats,
                        scheduler_log_q,
                        job.deadline_at,
                        task_start,
                    );
                }
                TaskId::Compression => {
                    run_compression_job(
                        sensor_q,
                        thermal_q,
                        downlink_q,
                        compression_log_q,
                        &mut scheduler_q_stats,
                        task_start,
                        &mut tx_seq,
                    );

                    record_task_completion(
                        job.id,
                        &mut compression_runtime.stats,
                        &mut scheduler_q_stats,
                        scheduler_log_q,
                        job.deadline_at,
                        task_start,
                    );
                }
                TaskId::Antenna => {
                    run_antenna_alignment_job(
                        task_start,
                        &system_state,
                        ant_log_q,
                        &mut scheduler_q_stats,
                    );

                    record_task_completion(
                        job.id,
                        &mut antenna_runtime.stats,
                        &mut scheduler_q_stats,
                        scheduler_log_q,
                        job.deadline_at,
                        task_start,
                    );
                }
                TaskId::Health => {
                    run_health_monitor_job(
                        start_time,
                        &system_state,
                        &mut scheduler_q_stats,
                        downlink_q,
                        health_log_q,
                    );

                    record_task_completion(
                        job.id,
                        &mut health_runtime.stats,
                        &mut scheduler_q_stats,
                        scheduler_log_q,
                        job.deadline_at,
                        task_start,
                    );
                }
                TaskId::Gyro => {
                    run_sensor_job(
                        sensor_q,
                        health_log_q,
                        &mut scheduler_q_stats,
                        task_start,
                        job.id,
                        &mut gyro_runtime.seq,
                    );

                    record_task_completion(
                        job.id,
                        &mut gyro_runtime.stats,
                        &mut scheduler_q_stats,
                        scheduler_log_q,
                        job.deadline_at,
                        task_start,
                    );
                }
                TaskId::Battery => {
                    run_sensor_job(
                        sensor_q,
                        health_log_q,
                        &mut scheduler_q_stats,
                        task_start,
                        job.id,
                        &mut battery_runtime.seq,
                    );

                    record_task_completion(
                        job.id,
                        &mut battery_runtime.stats,
                        &mut scheduler_q_stats,
                        scheduler_log_q,
                        job.deadline_at,
                        task_start,
                    );
                }
                _ => {}
            }

            cpu_stats.add_active(now.elapsed());
        }
        else {
            let next_release = gyro_runtime.next_release
                .min(battery_runtime.next_release)
                .min(compression_runtime.next_release)
                .min(health_runtime.next_release)
                .min(antenna_runtime.next_release)
                .min(cmd_exec_runtime.next_release);

            let idle_start = Instant::now();

            let wake_time = next_release - Duration::from_micros(100);
            thread::sleep(wake_time.saturating_duration_since(Instant::now()));

            cpu_stats.add_idle(idle_start.elapsed());
        }
        
        tick += 1;
    }

    SchedulerReport {
        cpu_stats,
        command_stats,
        scheduler_q_stats,

        gyro_timing,
        battery_timing,
        compression_timing,
        health_timing,
        antenna_timing,
        cmd_exec_timing,

        gyro_runtime,
        battery_runtime,
        compression_runtime,
        health_runtime,
        antenna_runtime,
        cmd_exec_runtime,
    }
}
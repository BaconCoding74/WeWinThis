use std::fs::{create_dir_all, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use crate::common::cpu_stats::CpuStats;
use crate::common::system_state::SystemState;
use crate::config::{AntLogSPSCBuffer, CommLogSPSCBuffer, CommandLogSPSCBuffer, CompressionLogSPSCBuffer, HealthLogSPSCBuffer, SchedulerLogSPSCBuffer, ThermalLogSPSCBuffer, LOGGER_FLUSH_INTERVAL_MS, LOGGER_IDLE_SLEEP_MS, LOG_FILE_PATH};
use crate::logger::helper::drain_queue;
use crate::logging::default::LogLevel;
use crate::reports::logger::LoggerReport;

pub fn run_logger_main(
    thermal_log_q: ThermalLogSPSCBuffer,
    comm_log_q: CommLogSPSCBuffer,
    scheduler_log_q: SchedulerLogSPSCBuffer,
    command_log_q: CommandLogSPSCBuffer,
    ant_log_q: AntLogSPSCBuffer,
    health_log_q: HealthLogSPSCBuffer,
    compression_log_q: CompressionLogSPSCBuffer,
    system_state: Arc<SystemState>,
) -> LoggerReport {
    let cpu_stats = CpuStats::new();
    create_dir_all("logs").ok();

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(LOG_FILE_PATH)
        .expect("Failed to open logger output file");

    let mut writer = BufWriter::new(file);
    let mut last_flush = Instant::now();

    loop {
        let loop_start = Instant::now();
        let mut drained_any = false;

        drain_queue(
            || comm_log_q.pop(),
            "COMM",
            |r| r.timestamp_ms,
            |r| r.level,
            |r| format!("{:?}", r),
            &mut writer,
            &mut drained_any,
        );

        drain_queue(
            || thermal_log_q.pop(),
            "THERM",
            |r| r.timestamp_ms,
            |r| r.level,
            |r| format!("{:?}", r),
            &mut writer,
            &mut drained_any,
        );

        drain_queue(
            || scheduler_log_q.pop(),
            "SCHED",
            |r| r.timestamp_ms,
            |r| r.level,
            |r| format!("{:?}", r),
            &mut writer,
            &mut drained_any,
        );

        drain_queue(
            || health_log_q.pop(),
            "HEALTH",
            |r| r.timestamp_ms,
            |_| LogLevel::Info,
            |r| format!("{:?}", r),
            &mut writer,
            &mut drained_any,
        );

        drain_queue(
            || ant_log_q.pop(),
            "ANT",
            |r| r.timestamp_ms,
            |_| LogLevel::Info,
            |r| format!("{:?}", r),
            &mut writer,
            &mut drained_any,
        );

        drain_queue(
            || command_log_q.pop(),
            "CMD",
            |r| r.timestamp_ms,
            |r| r.level,
            |r| format!("{:?}", r),
            &mut writer,
            &mut drained_any,
        );

        drain_queue(
            || compression_log_q.pop(),
            "COMP",
            |r| r.timestamp_ms,
            |r| r.level,
            |r| format!("{:?}", r),
            &mut writer,
            &mut drained_any,
        );

        if last_flush.elapsed() >= Duration::from_millis(LOGGER_FLUSH_INTERVAL_MS) {
            let _ = writer.flush();
            last_flush = Instant::now();
        }

        if system_state.stop.load(Ordering::Acquire) && !drained_any {
            let _ = writer.flush();
            break;
        }

        cpu_stats.add_active(last_flush.elapsed());

        if !drained_any {
            let idle_start = Instant::now();
            thread::sleep(Duration::from_millis(LOGGER_IDLE_SLEEP_MS));
            cpu_stats.add_idle(idle_start.elapsed());
        }
    }
    
    LoggerReport {
        cpu_stats,
    }
}
use std::sync::Arc;
use std::time::Instant;
use crate::common::metrics::elapsed_ms_u32;
use crate::common::scheduler_stats::SchedulerStats;
use crate::common::system_state::{SystemMode, SystemState};
use crate::config::{DownlinkSPSCBuffer, HealthLogSPSCBuffer, DOWNLINK_BUF_CAP, ENTER_DEGRADE_PERCENTAGE, EXIT_DEGRADE_PERCENTAGE};
use crate::logging::default::LogLevel;
use crate::logging::health::{HealthLogCode, HealthLogRecord};

#[inline]
fn push_health_log(
    health_log_q: &HealthLogSPSCBuffer,
    scheduler_stats: &mut SchedulerStats,
    start_time: Instant,
    level: LogLevel,
    code: HealthLogCode,
    value: i32,
) {
    let record = HealthLogRecord {
        level,
        timestamp_ms: elapsed_ms_u32(start_time),
        code,
        value,
    };

    if health_log_q.push(record).is_err() {
        scheduler_stats.health_log_dropped = scheduler_stats.health_log_dropped.saturating_add(1);
    }
}

pub fn run_health_monitor_job(
    start_time: Instant,
    system_state: &Arc<SystemState>,
    scheduler_stats: &mut SchedulerStats,
    downlink_q: &DownlinkSPSCBuffer,
    health_log_q: &HealthLogSPSCBuffer,
) {
    let fill_pct = if DOWNLINK_BUF_CAP > 0 {
        (downlink_q.len() * 100 / DOWNLINK_BUF_CAP) as i32
    } else {
        0
    };

    push_health_log(
        health_log_q,
        scheduler_stats,
        start_time,
        LogLevel::Info,
        HealthLogCode::DownlinkQueueFillPct,
        fill_pct,
    );

    match system_state.get_mode() {
        SystemMode::Normal => {
            if fill_pct >= ENTER_DEGRADE_PERCENTAGE {
                system_state.set_mode(SystemMode::Degraded);

                push_health_log(
                    health_log_q,
                    scheduler_stats,
                    start_time,
                    LogLevel::Warn,
                    HealthLogCode::EnterDegraded,
                    fill_pct,
                );
            }
        }
        SystemMode::Degraded => {
            if fill_pct <= EXIT_DEGRADE_PERCENTAGE {
                system_state.set_mode(SystemMode::Normal);

                push_health_log(
                    health_log_q,
                    scheduler_stats,
                    start_time,
                    LogLevel::Info,
                    HealthLogCode::ExitDegraded,
                    fill_pct,
                );
            }
        }
    }
}
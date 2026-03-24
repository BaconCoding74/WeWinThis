use std::sync::Arc;
use std::time::Instant;
use crate::common::metrics::elapsed_ms_u32;
use crate::common::system_state::{SystemMode, SystemState};
use crate::config::{DownlinkSPSCBuffer, HealthLogSPSCBuffer, DOWNLINK_BUF_CAP, ENTER_DEGRADE_PERCENTAGE, EXIT_DEGRADE_PERCENTAGE};
use crate::logging::health::{HealthLogCode, HealthLogRecord};

pub fn run_health_monitor_job(
    start_time: Instant,
    system_state: &Arc<SystemState>,
    downlink_q: &DownlinkSPSCBuffer,
    health_log_q: &HealthLogSPSCBuffer,
) {
    let now_ms = elapsed_ms_u32(start_time);

    let fill_pct = if DOWNLINK_BUF_CAP > 0 {
        (downlink_q.len() * 100 / DOWNLINK_BUF_CAP) as i32
    } else {
        0
    };

    let _ = health_log_q.push(HealthLogRecord {
        timestamp_ms: now_ms,
        code: HealthLogCode::Heartbeat,
        value: 1,
    });

    let _ = health_log_q.push(HealthLogRecord {
        timestamp_ms: now_ms,
        code: HealthLogCode::DownlinkQueueFillPct,
        value: fill_pct,
    });

    match system_state.get_mode() {
        SystemMode::Normal => {
            if fill_pct >= ENTER_DEGRADE_PERCENTAGE {
                system_state.set_mode(SystemMode::Degraded);

                let _ = health_log_q.push(HealthLogRecord {
                    timestamp_ms: now_ms,
                    code: HealthLogCode::EnterDegraded,
                    value: fill_pct,
                });
            }
        }
        SystemMode::Degraded => {
            if fill_pct <= EXIT_DEGRADE_PERCENTAGE {
                system_state.set_mode(SystemMode::Normal);

                let _ = health_log_q.push(HealthLogRecord {
                    timestamp_ms: now_ms,
                    code: HealthLogCode::ExitDegraded,
                    value: fill_pct,
                });
            }
        }
    }
}
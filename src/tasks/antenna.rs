use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use crate::common::metrics::elapsed_ms_u32;
use crate::common::scheduler_stats::SchedulerStats;
use crate::common::system_state::{SystemMode, SystemState};
use crate::config::{AntLogSPSCBuffer, DEGRADED_READY_THRESHOLD_DEG, DEGRADED_STEP_DEG, NORMAL_READY_THRESHOLD_DEG, NORMAL_STEP_DEG};
use crate::logging::antenna::{AntennaLogCode, AntennaLogRecord};
use crate::logging::default::LogLevel;

#[inline]
fn push_antenna_log(
    antenna_log_q: &AntLogSPSCBuffer,
    scheduler_stats: &mut SchedulerStats,
    start_time: Instant,
    level: LogLevel,
    code: AntennaLogCode,
    target_deg: i16,
    current_deg: i16,
    error_deg: i16,
    applied_step_deg: i16,
    ready: bool,
) {
    if antenna_log_q.push(AntennaLogRecord {
        level,
        timestamp_ms: elapsed_ms_u32(start_time),
        code,
        target_deg,
        current_deg,
        error_deg,
        applied_step_deg,
        ready,
    }).is_err() {
        scheduler_stats.antenna_log_dropped = scheduler_stats.antenna_log_dropped.saturating_add(1);
    }
}

pub fn run_antenna_alignment_job(
    start_time: Instant,
    system_state: &Arc<SystemState>,
    antenna_log_q: &AntLogSPSCBuffer,
    scheduler_stats: &mut SchedulerStats,
) {

    let prev_ready = system_state.antenna_ready.load(Ordering::Acquire);
    let current = system_state.antenna_current_deg.load(Ordering::Acquire);
    let target = system_state.antenna_target_deg.load(Ordering::Acquire);

    if !system_state.antenna_align_enabled.load(Ordering::Acquire) && prev_ready {
        system_state.antenna_ready.store(false, Ordering::Release);
        push_antenna_log(
            antenna_log_q,
            scheduler_stats,
            start_time,
            LogLevel::Warn,
            AntennaLogCode::AlignmentDisabled,
            target,
            current,
            target - current,
            0,
            false,
        );
        return;
    }

    let degraded = system_state.get_mode() == SystemMode::Degraded;

    let step_limit = if degraded {
        DEGRADED_STEP_DEG
    } else {
        NORMAL_STEP_DEG
    };

    let ready_threshold = if degraded {
        DEGRADED_READY_THRESHOLD_DEG
    } else {
        NORMAL_READY_THRESHOLD_DEG
    };


    let error = target - current;

    if error == 0 {
        let ready = true;
        system_state.antenna_ready.store(ready, Ordering::Release);

        if prev_ready != ready {
            push_antenna_log(
                antenna_log_q,
                scheduler_stats,
                start_time,
                LogLevel::Info,
                AntennaLogCode::TargetReached,
                target,
                current,
                0,
                0,
                ready,
            );
        }

        return;
    }

    let step = if error > step_limit {
        step_limit
    } else if error < -step_limit {
        -step_limit
    } else {
        error
    };

    let new_angle = current + step;
    system_state.antenna_current_deg.store(new_angle, Ordering::Release);

    let new_error = target - new_angle;
    let ready = new_error.abs() <= ready_threshold;
    system_state.antenna_ready.store(ready, Ordering::Release);

    let code = if prev_ready != ready {
        AntennaLogCode::ReadyChanged
    }
    else {
        AntennaLogCode::StepApplied
    };

    push_antenna_log(
        antenna_log_q,
        scheduler_stats,
        start_time,
        LogLevel::Info,
        code,
        target,
        new_angle,
        new_error,
        step,
        ready,
    );
}
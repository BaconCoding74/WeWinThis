use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use crate::common::metrics::elapsed_ms_u32;
use crate::common::system_state::{SystemMode, SystemState};
use crate::config::{AntLogSPSCBuffer, DEGRADED_READY_THRESHOLD_DEG, DEGRADED_STEP_DEG, NORMAL_READY_THRESHOLD_DEG, NORMAL_STEP_DEG};
use crate::logging::antenna::AntennaLogRecord;

pub fn run_antenna_alignment_job(
    start_time: Instant,
    system_state: &Arc<SystemState>,
    antenna_log_q: &AntLogSPSCBuffer,
) {

    let now_ms = elapsed_ms_u32(start_time);
    let prev_ready = system_state.antenna_ready.load(Ordering::Acquire);
    let current = system_state.antenna_current_deg.load(Ordering::Acquire);
    let target = system_state.antenna_target_deg.load(Ordering::Acquire);

    if !system_state.antenna_align_enabled.load(Ordering::Acquire) && prev_ready {
        system_state.antenna_ready.store(false, Ordering::Release);
        let _ = antenna_log_q.push(AntennaLogRecord {
            timestamp_ms: now_ms,
            target_deg: target,
            current_deg: current,
            error_deg: target - current,
            applied_step_deg: 0,
            ready: false,
        });
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
            let _ = antenna_log_q.push(AntennaLogRecord {
                timestamp_ms: now_ms,
                target_deg: target,
                current_deg: current,
                error_deg: 0,
                applied_step_deg: 0,
                ready,
            });
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

    let _ = antenna_log_q.push(AntennaLogRecord {
        timestamp_ms: now_ms,
        target_deg: target,
        current_deg: new_angle,
        error_deg: new_error,
        applied_step_deg: step,
        ready,
    });
}
use std::time::Instant;
use crate::common::metrics::elapsed_ms_u32;
use crate::common::scheduler_stats::{SchedulerStats};
use crate::common::tasks::TaskId;
use crate::config::SchedulerLogSPSCBuffer;
use crate::logging::default::LogLevel;
use crate::logging::scheduler::{SchedulerLogCode, SchedulerLogRecord};

#[inline]
pub fn log_scheduler(
    q_stats: &mut SchedulerStats,
    log_q: &SchedulerLogSPSCBuffer,
    start_time: Instant,
    level: LogLevel,
    code: SchedulerLogCode,
    task_id: TaskId,
    value: i32,
) {
    if log_q.push(SchedulerLogRecord {
        level,
        timestamp_ms: elapsed_ms_u32(start_time),
        code,
        task_id,
        value,
    }).is_err() {
        q_stats.scheduler_log_dropped += 1;
    };
}
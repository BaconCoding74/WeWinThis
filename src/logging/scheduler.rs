use crate::common::tasks::TaskId;
use crate::logging::default::LogLevel;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerLogCode {
    ReadyQueueFull = 2000,
    CompletionDeadlineMiss = 2001,
}

#[derive(Debug, Clone, Copy)]
pub struct SchedulerLogRecord {
    pub level: LogLevel,
    pub timestamp_ms: u32,
    pub code: SchedulerLogCode,
    pub task_id: TaskId,
    pub value: i32,
}
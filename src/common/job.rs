use std::time::Instant;
use crate::common::tasks::TaskId;

#[derive(Debug, Clone, Copy)]
pub struct Job {
    pub id: TaskId,
    pub priority: u8,
    pub release_tick: u64,
    pub seq: u64,
    pub released_at: Instant,
    pub expected_at: Instant,
    pub deadline_at: Instant,
}

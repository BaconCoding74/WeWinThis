use std::time::{Duration, Instant};
use crate::common::metrics::TaskStats;
use crate::common::tasks::TaskId;

#[derive(Debug, Clone, Copy)]
pub struct TaskTiming {
    pub period: Duration,
    pub deadline: Duration,
    pub priority: u8,
}

pub fn task_timing(task_id: TaskId) -> TaskTiming {
    match task_id {
        TaskId::Thermal => TaskTiming {
            period: Duration::from_millis(20),
            deadline: Duration::from_millis(18),
            priority: 1,
        },

        TaskId::CommandExec => TaskTiming {
            period: Duration::from_millis(20),
            deadline: Duration::from_millis(15),
            priority: 2,
        },

        TaskId::Gyro => TaskTiming {
            period: Duration::from_millis(25),
            deadline: Duration::from_millis(23),
            priority: 3,
        },

        TaskId::Battery => TaskTiming {
            period: Duration::from_millis(40),
            deadline: Duration::from_millis(35),
            priority: 4,
        },

        TaskId::Health => TaskTiming {
            period: Duration::from_millis(100),
            deadline: Duration::from_millis(30),
            priority: 5,
        },

        TaskId::Antenna => TaskTiming {
            period: Duration::from_millis(50),
            deadline: Duration::from_millis(20),
            priority: 6,
        },

        TaskId::Compression => TaskTiming {
            period: Duration::from_millis(50),
            deadline: Duration::from_millis(30),
            priority: 7,
        },
    }
}

#[derive(Debug)]
pub struct TaskRuntime {
    pub next_release: Instant,
    pub seq: u64,
    pub stats: TaskStats,
}

impl TaskRuntime {
    pub fn new(start: Instant) -> TaskRuntime {
        TaskRuntime {
            next_release: start,
            seq: 0,
            stats: TaskStats::new()
        }
    }
}
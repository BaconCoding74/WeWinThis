use std::time::{Duration, Instant};
use crate::common::timing::TaskTiming;

#[derive(Debug, Default)]
pub struct TaskStats {
    releases: u64,
    completions: u64,
    deadline_misses: u64,

    total_drift: Duration,
    max_drift: Duration,

    total_latency: Duration,
    max_latency: Duration,
    min_latency: Duration,
}

impl TaskStats {
    pub const fn new() -> Self {
        Self {
            releases: 0,
            completions: 0,
            deadline_misses: 0,

            total_drift: Duration::ZERO,
            max_drift: Duration::ZERO,

            total_latency: Duration::ZERO,
            max_latency: Duration::ZERO,
            min_latency: Duration::MAX,
        }
    }
    pub fn record_release(&mut self, expected_release: Instant, actual_release: Instant) {
        self.releases += 1;

        let drift = actual_release
            .checked_duration_since(expected_release)
            .unwrap_or(Duration::ZERO);

        self.total_drift += drift;
        self.max_drift = self.max_drift.max(drift);
    }

    pub fn record_completion(
        &mut self,
        start_time: Instant,
        finish_time: Instant,
        deadline_missed: bool,
    ) {
        self.completions += 1;

        let latency = finish_time
            .checked_duration_since(start_time)
            .unwrap_or(Duration::ZERO);

        self.total_latency += latency;
        self.max_latency = self.max_latency.max(latency);
        self.min_latency = self.min_latency.min(latency);

        if deadline_missed {
            self.deadline_misses += 1;
        }
    }

    fn avg_drift(&self) -> Duration {
        if self.releases == 0 {
            Duration::ZERO
        } else {
            duration_div(self.total_drift, self.releases)
        }
    }

    fn avg_latency(&self) -> Duration {
        if self.completions == 0 {
            Duration::ZERO
        } else {
            duration_div(self.total_latency, self.completions)
        }
    }

    fn jitter(&self) -> Duration {
        if self.completions == 0 {
            Duration::ZERO
        } else {
            self.max_latency.saturating_sub(self.min_latency)
        }
    }
}

pub fn elapsed_ms_u32(start: Instant) -> u32 {
    start.elapsed().as_millis().min(u32::MAX as u128) as u32
}

pub fn duration_div(d: Duration, n: u64) -> Duration {
    Duration::from_nanos((d.as_nanos() / n as u128) as u64)
}

pub fn print_task_report(name: &str, task: &TaskTiming, stats: &TaskStats, is_periodic: bool) {
    println!("Task {name}");

    if is_periodic {
        println!("  deadline        : {:?}", task.deadline);
        println!("  period          : {:?}", task.period);

        println!("  releases        : {}", stats.releases);
        println!("  completions     : {}", stats.completions);
        println!("  deadline_misses : {}", stats.deadline_misses);

        println!("  avg drift       : {:?}", stats.avg_drift());
        println!("  max drift       : {:?}", stats.max_drift);
    }

    println!("  avg latency     : {:?}", stats.avg_latency());
    println!("  max latency     : {:?}", stats.max_latency);
    println!("  min latency     : {:?}", stats.min_latency);

    println!("  jitter          : {:?}", stats.jitter());
}
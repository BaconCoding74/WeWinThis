use std::time::{Duration, Instant};
use crate::common::metrics::duration_div;

#[derive(Debug, Default)]
pub struct CommandStats {
    commands_received: u64,
    commands_executed: u64,
    responses_enqueued: u64,
    response_drops: u64,
    log_drops: u64,

    total_exec_latency: Duration,
    max_exec_latency: Duration,
    min_exec_latency: Duration,

    total_response_latency: Duration,
    max_response_latency: Duration,
    min_response_latency: Duration,
}

impl CommandStats {
    pub const fn new() -> Self {
        Self {
            commands_received: 0,
            commands_executed: 0,
            responses_enqueued: 0,
            response_drops: 0,
            log_drops: 0,

            total_exec_latency: Duration::ZERO,
            max_exec_latency: Duration::ZERO,
            min_exec_latency: Duration::MAX,

            total_response_latency: Duration::ZERO,
            max_response_latency: Duration::ZERO,
            min_response_latency: Duration::MAX,
        }
    }

    pub fn record_log_dropped(&mut self) {self.log_drops +=1; }

    pub fn record_received(&mut self) {
        self.commands_received += 1;
    }

    pub fn record_execution(&mut self, start_time: Instant, finish_time: Instant) {
        self.commands_executed += 1;

        let latency = finish_time
            .checked_duration_since(start_time)
            .unwrap_or(Duration::ZERO);

        self.total_exec_latency += latency;
        self.max_exec_latency = self.max_exec_latency.max(latency);
        self.min_exec_latency = self.min_exec_latency.min(latency);
    }

    pub fn record_response_enqueued(&mut self, received_time: Instant, enqueued_time: Instant) {
        self.responses_enqueued += 1;

        let latency = enqueued_time
            .checked_duration_since(received_time)
            .unwrap_or(Duration::ZERO);

        self.total_response_latency += latency;
        self.max_response_latency = self.max_response_latency.max(latency);
        self.min_response_latency = self.min_response_latency.min(latency);
    }

    pub fn record_response_drop(&mut self) {
        self.response_drops += 1;
    }

    fn avg_exec_latency(&self) -> Duration {
        if self.commands_executed == 0 {
            Duration::ZERO
        } else {
            duration_div(self.total_exec_latency, self.commands_executed)
        }
    }

    fn avg_response_latency(&self) -> Duration {
        if self.responses_enqueued == 0 {
            Duration::ZERO
        } else {
            duration_div(self.total_response_latency, self.responses_enqueued)
        }
    }

    fn exec_jitter(&self) -> Duration {
        if self.commands_executed == 0 {
            Duration::ZERO
        } else {
            self.max_exec_latency.saturating_sub(self.min_exec_latency)
        }
    }

    fn response_jitter(&self) -> Duration {
        if self.responses_enqueued == 0 {
            Duration::ZERO
        } else {
            self.max_response_latency
                .saturating_sub(self.min_response_latency)
        }
    }

    fn effective_min_exec_latency(&self) -> Duration {
        if self.commands_executed == 0 {
            Duration::ZERO
        } else {
            self.min_exec_latency
        }
    }

    fn effective_min_response_latency(&self) -> Duration {
        if self.responses_enqueued == 0 {
            Duration::ZERO
        } else {
            self.min_response_latency
        }
    }
}

pub fn print_command_report(name: &str, stats: &CommandStats) {
    println!("Command Task {name}");
    println!("  log drops             : {}", stats.log_drops);

    println!("  received              : {}", stats.commands_received);
    println!("  executed              : {}", stats.commands_executed);
    println!("  responses enqueued    : {}", stats.responses_enqueued);
    println!("  response drops        : {}", stats.response_drops);

    println!("  avg exec latency      : {:?}", stats.avg_exec_latency());
    println!("  max exec latency      : {:?}", stats.max_exec_latency);
    println!("  min exec latency      : {:?}", stats.effective_min_exec_latency());
    println!("  exec jitter           : {:?}", stats.exec_jitter());

    println!("  avg response latency  : {:?}", stats.avg_response_latency());
    println!("  max response latency  : {:?}", stats.max_response_latency);
    println!("  min response latency  : {:?}", stats.effective_min_response_latency());
    println!("  response jitter       : {:?}", stats.response_jitter());
}
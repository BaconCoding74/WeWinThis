use std::time::{Duration, Instant};
use crate::common::metrics::duration_div;
use crate::config::{COMM_RESPONSE_DEADLINE, DOWNLINK_PREP_BUDGET};

#[derive(Debug)]
pub struct CommunicationStats {
    packet_received: u64,
    packet_sent: u64,

    log_dropped: u64,
    command_dropped: u64,

    bad_packets: u64,
    command_rejected: u64,
    socket_errors: u64,

    connect_successes: u64,
    downlink_init_misses: u64,
    downlink_late: u64,

    cmd_responses_sent: u64,

    total_connect_latency: Duration,
    max_connect_latency: Duration,
    min_connect_latency: Duration,

    total_packet_downlink_latency: Duration,
    max_packet_downlink_latency: Duration,
    min_packet_downlink_latency: Duration,

    total_response_queue_latency: Duration,
    max_response_queue_latency: Duration,
    min_response_queue_latency: Duration,

    total_command_response_sent_latency: Duration,
    max_command_response_sent_latency: Duration,
    min_command_response_sent_latency: Duration,
    command_response_deadline_count: u64,

    total_downlink_late_latency: Duration,
    max_downlink_late_latency: Duration,
    min_downlink_late_latency: Duration,

    total_downlink_prep_latency: Duration,
    max_downlink_prep_latency: Duration,
    min_downlink_prep_latency: Duration,
}

impl CommunicationStats {
    pub const fn new() -> Self {
        Self {
            packet_received: 0,
            packet_sent: 0,

            log_dropped: 0,
            command_dropped: 0,

            bad_packets: 0,
            command_rejected: 0,
            socket_errors: 0,

            connect_successes: 0,
            downlink_init_misses: 0,
            downlink_late: 0,

            cmd_responses_sent: 0,

            total_connect_latency: Duration::ZERO,
            max_connect_latency: Duration::ZERO,
            min_connect_latency: Duration::MAX,

            total_packet_downlink_latency: Duration::ZERO,
            max_packet_downlink_latency: Duration::ZERO,
            min_packet_downlink_latency: Duration::MAX,

            total_response_queue_latency: Duration::ZERO,
            max_response_queue_latency: Duration::ZERO,
            min_response_queue_latency: Duration::MAX,
            
            total_command_response_sent_latency: Duration::ZERO,
            max_command_response_sent_latency: Duration::ZERO,
            min_command_response_sent_latency: Duration::MAX,
            command_response_deadline_count: 0,

            total_downlink_late_latency: Duration::ZERO,
            max_downlink_late_latency: Duration::ZERO,
            min_downlink_late_latency: Duration::MAX,

            total_downlink_prep_latency: Duration::ZERO,
            max_downlink_prep_latency: Duration::ZERO,
            min_downlink_prep_latency: Duration::MAX,
        }
    }

    pub fn record_log_dropped(&mut self) {self.log_dropped += 1;}

    pub fn record_command_dropped(&mut self) {self.command_dropped += 1;}

    pub fn record_packet_received(&mut self) {
        self.packet_received += 1;
    }

    pub fn record_packet_sent(&mut self, enqueued_at: Instant, sent_at: Instant) {
        self.packet_sent += 1;

        let latency = sent_at
            .checked_duration_since(enqueued_at)
            .unwrap_or(Duration::ZERO);

        self.total_packet_downlink_latency += latency;
        self.max_packet_downlink_latency = self.max_packet_downlink_latency.max(latency);
        self.min_packet_downlink_latency = self.min_packet_downlink_latency.min(latency);
    }

    pub fn record_downlink_prep(&mut self, window_open_at: Instant, sent_at: Instant) {
        let latency = sent_at.duration_since(window_open_at);

        if latency > self.max_downlink_prep_latency {
            self.max_downlink_prep_latency = latency;
        }

        if self.min_downlink_prep_latency.is_zero() || latency < self.min_downlink_prep_latency {
            self.min_downlink_prep_latency = latency;
        }

        self.total_downlink_prep_latency += latency;

        if latency > DOWNLINK_PREP_BUDGET {
            self.downlink_late += 1;
        }
    }

    pub fn record_bad_packet(&mut self) {
        self.bad_packets += 1;
    }

    pub fn record_command_rejected(&mut self) {
        self.command_rejected += 1;
    }

    pub fn record_socket_error(&mut self) {
        self.socket_errors += 1;
    }

    pub fn record_downlink_init_miss(&mut self) {
        self.downlink_init_misses += 1;
    }

    pub fn record_connect_success(&mut self, connect_start: Instant, connected_at: Instant) {
        self.connect_successes += 1;

        let latency = connected_at
            .checked_duration_since(connect_start)
            .unwrap_or(Duration::ZERO);

        self.total_connect_latency += latency;
        self.max_connect_latency = self.max_connect_latency.max(latency);
        self.min_connect_latency = self.min_connect_latency.min(latency);
    }

    pub fn record_downlink_late(&mut self, window_open_at: Instant, observed_at: Instant) {
        self.downlink_late += 1;

        let latency = observed_at
            .checked_duration_since(window_open_at)
            .unwrap_or(Duration::ZERO);

        self.total_downlink_late_latency += latency;
        self.max_downlink_late_latency = self.max_downlink_late_latency.max(latency);
        self.min_downlink_late_latency = self.min_downlink_late_latency.min(latency);
    }

    pub fn record_command_response_sent(
        &mut self,
        enqueued_at: Instant,
        sent_at: Instant,
        cmd_rx_at: Instant,
    ) {
        self.cmd_responses_sent += 1;

        let queue_latency = sent_at
            .checked_duration_since(enqueued_at)
            .unwrap_or(Duration::ZERO);

        self.total_response_queue_latency += queue_latency;
        self.max_response_queue_latency = self.max_response_queue_latency.max(queue_latency);
        self.min_response_queue_latency = self.min_response_queue_latency.min(queue_latency);

        let end_to_end = sent_at
            .checked_duration_since(cmd_rx_at)
            .unwrap_or(Duration::ZERO);
        
        if end_to_end > COMM_RESPONSE_DEADLINE {
            self.command_response_deadline_count += 1;
        }

        self.total_command_response_sent_latency += end_to_end;
        self.max_command_response_sent_latency =
            self.max_command_response_sent_latency.max(end_to_end);
        self.min_command_response_sent_latency =
            self.min_command_response_sent_latency.min(end_to_end);
    }

    fn avg_connect_latency(&self) -> Duration {
        if self.connect_successes == 0 {
            Duration::ZERO
        } else {
            duration_div(self.total_connect_latency, self.connect_successes)
        }
    }

    fn avg_response_queue_latency(&self) -> Duration {
        if self.cmd_responses_sent == 0 {
            Duration::ZERO
        } else {
            duration_div(self.total_response_queue_latency, self.cmd_responses_sent)
        }
    }

    fn avg_packet_downlink_latency(&self) -> Duration {
        if self.cmd_responses_sent == 0 {
            Duration::ZERO
        } else {
            duration_div(self.total_packet_downlink_latency, self.packet_sent)
        }
    }

    fn avg_command_response_sent_latency(&self) -> Duration {
        if self.cmd_responses_sent == 0 {
            Duration::ZERO
        } else {
            duration_div(
                self.total_command_response_sent_latency,
                self.cmd_responses_sent,
            )
        }
    }

    fn avg_downlink_late_latency(&self) -> Duration {
        if self.downlink_late == 0 {
            Duration::ZERO
        } else {
            duration_div(self.total_downlink_late_latency, self.downlink_late)
        }
    }

    fn effective_min_connect_latency(&self) -> Duration {
        if self.connect_successes == 0 {
            Duration::ZERO
        } else {
            self.min_connect_latency
        }
    }

    fn effective_min_response_queue_latency(&self) -> Duration {
        if self.cmd_responses_sent == 0 {
            Duration::ZERO
        } else {
            self.min_response_queue_latency
        }
    }

    fn effective_min_packet_downlink_latency(&self) -> Duration {
        if self.packet_sent == 0 {
            Duration::ZERO
        } else {
            self.min_packet_downlink_latency
        }
    }

    fn effective_min_command_response_sent_latency(&self) -> Duration {
        if self.cmd_responses_sent == 0 {
            Duration::ZERO
        } else {
            self.min_command_response_sent_latency
        }
    }

    fn effective_min_downlink_late_latency(&self) -> Duration {
        if self.downlink_late == 0 {
            Duration::ZERO
        } else {
            self.min_downlink_late_latency
        }
    }

    fn connect_jitter(&self) -> Duration {
        if self.connect_successes == 0 {
            Duration::ZERO
        } else {
            self.max_connect_latency
                .saturating_sub(self.effective_min_connect_latency())
        }
    }

    fn response_queue_jitter(&self) -> Duration {
        if self.cmd_responses_sent == 0 {
            Duration::ZERO
        } else {
            self.max_response_queue_latency
                .saturating_sub(self.effective_min_response_queue_latency())
        }
    }

    fn command_response_sent_jitter(&self) -> Duration {
        if self.cmd_responses_sent == 0 {
            Duration::ZERO
        } else {
            self.max_command_response_sent_latency
                .saturating_sub(self.effective_min_command_response_sent_latency())
        }
    }

    fn packet_downlink_jitter(&self) -> Duration {
        if self.packet_sent == 0 {
            Duration::ZERO
        } else {
            self.max_packet_downlink_latency
                .saturating_sub(self.effective_min_packet_downlink_latency())
        }
    }

    fn downlink_late_jitter(&self) -> Duration {
        if self.downlink_late == 0 {
            Duration::ZERO
        } else {
            self.max_downlink_late_latency
                .saturating_sub(self.effective_min_downlink_late_latency())
        }
    }

    fn downlink_prep_jitter(&self) -> Duration {
        self.max_downlink_late_latency.saturating_sub(self.min_downlink_prep_latency)
    }
}

pub fn print_comm_report(stats: &CommunicationStats) {
    println!("Communication Task");
    println!("  cmd -> response deadline        : {:?}", COMM_RESPONSE_DEADLINE);
    println!("  command dropped                 : {:?}", stats.command_dropped);
    println!("  log dropped                     : {:?}", stats.log_dropped);

    println!("  packet_received                 : {}", stats.packet_received);
    println!("  packet_sent                     : {}", stats.packet_sent);
    println!("  bad_packets                     : {}", stats.bad_packets);
    println!("  command_rejected                : {}", stats.command_rejected);
    println!("  socket_errors                   : {}", stats.socket_errors);

    println!("  connect_successes               : {}", stats.connect_successes);
    println!("  downlink_init_misses            : {}", stats.downlink_init_misses);
    println!("  downlink_late_events            : {}", stats.downlink_late);
    println!("  cmd_responses_sent              : {}", stats.cmd_responses_sent);

    println!("  avg connect latency             : {:?}", stats.avg_connect_latency());
    println!("  max connect latency             : {:?}", stats.max_connect_latency);
    println!("  min connect latency             : {:?}", stats.effective_min_connect_latency());
    println!("  connect jitter                  : {:?}", stats.connect_jitter());

    println!("  avg packet downlink latency      : {:?}", stats.avg_packet_downlink_latency());
    println!("  max packet downlink latency      : {:?}", stats.max_packet_downlink_latency);
    println!("  min packet downlink latency      : {:?}", stats.effective_min_packet_downlink_latency());
    println!("  packet downlink jitter           : {:?}", stats.packet_downlink_jitter());

    println!("  avg response queue latency      : {:?}", stats.avg_response_queue_latency());
    println!("  max response queue latency      : {:?}", stats.max_response_queue_latency);
    println!("  min response queue latency      : {:?}", stats.effective_min_response_queue_latency());
    println!("  response queue jitter           : {:?}", stats.response_queue_jitter());

    println!("  avg cmd->response sent latency  : {:?}", stats.avg_command_response_sent_latency());
    println!("  max cmd->response sent latency  : {:?}", stats.max_command_response_sent_latency);
    println!("  min cmd->response sent latency  : {:?}", stats.effective_min_command_response_sent_latency());
    println!("  cmd->response sent jitter       : {:?}", stats.command_response_sent_jitter());
    println!("  cmd->response sent deadline miss: {:?}", stats.command_response_deadline_count);

    println!("  avg downlink late latency       : {:?}", stats.avg_downlink_late_latency());
    println!("  max downlink late latency       : {:?}", stats.max_downlink_late_latency);
    println!("  min downlink late latency       : {:?}", stats.effective_min_downlink_late_latency());
    println!("  downlink late jitter            : {:?}", stats.downlink_late_jitter());

    println!("  max downlink prep latency       : {:?}", stats.max_downlink_prep_latency);
    println!("  min downlink prep latency       : {:?}", stats.min_downlink_prep_latency);
    println!("  downlink prep jitter            : {:?}", stats.downlink_prep_jitter());
}
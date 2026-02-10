use std::time::Instant;

pub struct PerformanceMetrics {
    pub packets_sent: u64,
    pub total_bytes_sent: u64,
    pub send_latency_us: u128,
    pub min_latency_us: u128,
    pub max_latency_us: u128,
    pub edge_case_count: u64,
    pub start_time: Instant,
    pub commands_received: u64,
    pub commands_executed: u64,
    pub commands_overdue: u64,
    pub faults_injected: u64,
    pub safety_alerts: u64,
    pub recovery_times_ms: Vec<u128>,
    pub scheduling_drift_us: Vec<i128>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            packets_sent: 0,
            total_bytes_sent: 0,
            send_latency_us: 0,
            min_latency_us: u128::MAX,
            max_latency_us: 0,
            edge_case_count: 0,
            start_time: Instant::now(),
            commands_received: 0,
            commands_executed: 0,
            commands_overdue: 0,
            faults_injected: 0,
            safety_alerts: 0,
            recovery_times_ms: Vec::new(),
            scheduling_drift_us: Vec::new(),
        }
    }

    pub fn record_send(&mut self, latency_us: u128, bytes: usize, is_edge_case: bool) {
        self.packets_sent += 1;
        self.total_bytes_sent += bytes as u64;
        self.send_latency_us += latency_us;
        self.min_latency_us = self.min_latency_us.min(latency_us);
        self.max_latency_us = self.max_latency_us.max(latency_us);
        if is_edge_case {
            self.edge_case_count += 1;
        }
    }

    pub fn record_command_received(&mut self) {
        self.commands_received += 1;
    }

    pub fn record_command_executed(&mut self, was_overdue: bool) {
        self.commands_executed += 1;
        if was_overdue {
            self.commands_overdue += 1;
        }
    }

    pub fn record_fault_injected(&mut self) {
        self.faults_injected += 1;
    }

    pub fn record_safety_alert(&mut self) {
        self.safety_alerts += 1;
    }

    pub fn record_recovery_time(&mut self, time_ms: u128) {
        self.recovery_times_ms.push(time_ms);
    }

    pub fn record_scheduling_drift(&mut self, drift_us: i128) {
        self.scheduling_drift_us.push(drift_us);
    }

    pub fn report(&self) {
        let elapsed = self.start_time.elapsed();
        let avg_latency = if self.packets_sent > 0 {
            self.send_latency_us / self.packets_sent as u128
        } else {
            0
        };

        let avg_drift = if !self.scheduling_drift_us.is_empty() {
            self.scheduling_drift_us.iter().sum::<i128>() / self.scheduling_drift_us.len() as i128
        } else {
            0
        };

        let avg_recovery = if !self.recovery_times_ms.is_empty() {
            self.recovery_times_ms.iter().sum::<u128>() / self.recovery_times_ms.len() as u128
        } else {
            0
        };

        println!("\n{}", "=".repeat(60));
        println!("MOCK OCS PERFORMANCE REPORT");
        println!("{}", "=".repeat(60));
        println!("Duration: {:?}", elapsed);
        println!("\n--- Telemetry ---");
        println!("Packets sent: {}", self.packets_sent);
        println!("Total bytes sent: {}", self.total_bytes_sent);
        println!(
            "Packets/second: {:.2}",
            self.packets_sent as f64 / elapsed.as_secs_f64()
        );
        println!("Average send latency: {} μs", avg_latency);
        println!("Min send latency: {} μs", self.min_latency_us);
        println!("Max send latency: {} μs", self.max_latency_us);
        println!("Edge cases injected: {}", self.edge_case_count);
        println!("\n--- Command Executor ---");
        println!("Commands received: {}", self.commands_received);
        println!("Commands executed: {}", self.commands_executed);
        println!("Commands overdue: {}", self.commands_overdue);
        println!("\n--- Fault Management ---");
        println!("Faults injected: {}", self.faults_injected);
        println!("Safety alerts: {}", self.safety_alerts);
        println!(
            "Average recovery time: {} ms (target: <200ms)",
            avg_recovery
        );
        println!("\n--- Scheduling ---");
        println!("Average scheduling drift: {} μs", avg_drift);
        let drift_status = if avg_drift.abs() < 1000 {
            "Within acceptable bounds"
        } else {
            "EXCESSIVE DRIFT DETECTED"
        };
        println!("{}", drift_status);
        println!("{}", "=".repeat(60));
    }
}

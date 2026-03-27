mod command;
mod command_schedule;
mod constants;
mod gcs;
mod logger;
mod network;
mod system_state;
mod telemetry;
mod thermal;

use std::{
    sync::Arc,
    sync::Mutex,
    sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::{
    gcs::{create_shared_loggers, tcp_listener},
    system_state::SystemState,
};

#[derive(Debug, Default)]
pub struct GcsStats {
    commands_sent: AtomicUsize,
    commands_acked: AtomicUsize,
    commands_rejected: AtomicUsize,
    commands_interlock: AtomicUsize,
    telemetry_received: AtomicUsize,
    faults_received: AtomicUsize,
    total_latency_ns: AtomicU64,
    min_latency_ns: AtomicU64,
    max_latency_ns: AtomicU64,
    total_jitter_ns: AtomicU64,
    max_jitter_ns: AtomicU64,
    total_telemetry_latency_ns: AtomicU64,
    min_telemetry_latency_ns: AtomicU64,
    max_telemetry_latency_ns: AtomicU64,
    last_send_time: Mutex<Option<Instant>>,
    last_telemetry_time: Mutex<Option<Instant>>,
}

impl GcsStats {
    pub fn record_command_sent(&self) {
        let mut last_send = self.last_send_time.lock().unwrap();
        *last_send = Some(Instant::now());
        self.commands_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_command_acked(&self) {
        let last_send = self.last_send_time.lock().unwrap();
        if let Some(send_time) = *last_send {
            let latency = send_time.elapsed().as_nanos() as u64;

            self.total_latency_ns.fetch_add(latency, Ordering::Relaxed);
            let count = self.commands_acked.load(Ordering::Relaxed) as u64 + 1;

            let current_min = self.min_latency_ns.load(Ordering::Relaxed);
            if current_min == 0 || latency < current_min {
                self.min_latency_ns.store(latency, Ordering::Relaxed);
            }

            let current_max = self.max_latency_ns.load(Ordering::Relaxed);
            if latency > current_max {
                self.max_latency_ns.store(latency, Ordering::Relaxed);
            }

            if count > 1 {
                let prev_latency = self.total_latency_ns.load(Ordering::Relaxed) / (count - 1);
                let jitter = latency.abs_diff(prev_latency);
                self.total_jitter_ns.fetch_add(jitter, Ordering::Relaxed);

                let current_max_jitter = self.max_jitter_ns.load(Ordering::Relaxed);
                if jitter > current_max_jitter {
                    self.max_jitter_ns.store(jitter, Ordering::Relaxed);
                }
            }
        }
        drop(last_send);

        self.commands_acked.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_command_rejected(&self) {
        self.commands_rejected.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_command_interlock(&self) {
        self.commands_interlock.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_telemetry(&self) {
        let now = Instant::now();
        let mut last_telemetry = self.last_telemetry_time.lock().unwrap();

        if let Some(last_time) = *last_telemetry {
            let latency = now.duration_since(last_time).as_nanos() as u64;
            self.total_telemetry_latency_ns
                .fetch_add(latency, Ordering::Relaxed);

            let current_min = self.min_telemetry_latency_ns.load(Ordering::Relaxed);
            if current_min == 0 || latency < current_min {
                self.min_telemetry_latency_ns
                    .store(latency, Ordering::Relaxed);
            }

            let current_max = self.max_telemetry_latency_ns.load(Ordering::Relaxed);
            if latency > current_max {
                self.max_telemetry_latency_ns
                    .store(latency, Ordering::Relaxed);
            }
        }
        *last_telemetry = Some(now);

        self.telemetry_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_fault(&self) {
        self.faults_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn print_summary(&self) {
        let sent = self.commands_sent.load(Ordering::Relaxed);
        let acked = self.commands_acked.load(Ordering::Relaxed);
        let rejected = self.commands_rejected.load(Ordering::Relaxed);
        let interlock = self.commands_interlock.load(Ordering::Relaxed);
        let telemetry = self.telemetry_received.load(Ordering::Relaxed);
        let faults = self.faults_received.load(Ordering::Relaxed);

        let total_latency = self.total_latency_ns.load(Ordering::Relaxed);
        let min_latency = self.min_latency_ns.load(Ordering::Relaxed);
        let max_latency = self.max_latency_ns.load(Ordering::Relaxed);
        let total_jitter = self.total_jitter_ns.load(Ordering::Relaxed);
        let max_jitter = self.max_jitter_ns.load(Ordering::Relaxed);

        let avg_latency_ns = if acked > 0 {
            total_latency / acked as u64
        } else {
            0
        };
        let avg_jitter_ns = if acked > 1 {
            total_jitter / (acked as u64 - 1)
        } else {
            0
        };

        let total_telemetry_latency = self.total_telemetry_latency_ns.load(Ordering::Relaxed);
        let min_telemetry_latency = self.min_telemetry_latency_ns.load(Ordering::Relaxed);
        let max_telemetry_latency = self.max_telemetry_latency_ns.load(Ordering::Relaxed);
        let avg_telemetry_latency_ns = if telemetry > 0 {
            total_telemetry_latency / telemetry as u64
        } else {
            0
        };

        println!("\n==========================================");
        println!("============= GCS Summary ================");
        println!("==========================================");
        println!("Task Command Transmission");
        println!("  commands sent            : {}", sent);
        println!("  commands acked           : {}", acked);
        println!("  commands rejected        : {}", rejected);
        println!("  commands interlock       : {}", interlock);
        println!(
            "  avg latency              : {:.3}µs",
            avg_latency_ns as f64 / 1000.0
        );
        println!(
            "  max latency              : {:.3}µs",
            max_latency as f64 / 1000.0
        );
        println!(
            "  min latency              : {:.3}µs",
            min_latency as f64 / 1000.0
        );
        println!(
            "  avg jitter               : {:.3}µs",
            avg_jitter_ns as f64 / 1000.0
        );
        println!(
            "  max jitter               : {:.3}µs",
            max_jitter as f64 / 1000.0
        );
        println!("Task Telemetry Reception");
        println!("  packets received         : {}", telemetry);
        println!("  fault packets            : {}", faults);
        println!(
            "  avg packet interval      : {:.3}ms",
            avg_telemetry_latency_ns as f64 / 1_000_000.0
        );
        println!(
            "  max packet interval      : {:.3}ms",
            max_telemetry_latency as f64 / 1_000_000.0
        );
        println!(
            "  min packet interval      : {:.3}ms",
            min_telemetry_latency as f64 / 1_000_000.0
        );
        println!("==========================================\n");
    }
}

fn main() {
    let system_state = Arc::new(SystemState::new());
    let gcs_stats = Arc::new(GcsStats::default());

    let (command_logger, telemetry_logger, system_state_logger, performance_logger, fault_logger) =
        create_shared_loggers();

    let telemetry_backlog = Arc::new(AtomicU32::new(0));

    let state_clone = Arc::clone(&system_state);
    let stats_clone = Arc::clone(&gcs_stats);
    let command_logger_clone = Arc::clone(&command_logger);
    let telemetry_logger_clone = Arc::clone(&telemetry_logger);
    let system_state_logger_clone = Arc::clone(&system_state_logger);
    let fault_logger_clone = Arc::clone(&fault_logger);
    let performance_logger_clone = Arc::clone(&performance_logger);
    let backlog_clone = Arc::clone(&telemetry_backlog);

    thread::spawn(move || {
        tcp_listener(
            state_clone,
            stats_clone,
            command_logger_clone,
            telemetry_logger_clone,
            system_state_logger_clone,
            fault_logger_clone,
            performance_logger_clone,
            backlog_clone,
        );
    });

    println!("GCS started. Press Ctrl+C to exit...");

    let stats_for_shutdown = Arc::clone(&gcs_stats);
    let running = Arc::new(AtomicU32::new(1));
    let running_for_signal = Arc::clone(&running);

    ctrlc::set_handler(move || {
        if running_for_signal.swap(0, Ordering::SeqCst) == 1 {
            stats_for_shutdown.print_summary();
            std::process::exit(0);
        }
    })
    .expect("Error setting Ctrl+C handler");

    while running.load(Ordering::Relaxed) == 1 {
        thread::sleep(Duration::from_millis(100));
    }
}

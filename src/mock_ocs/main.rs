mod command;
mod metrics;
mod telemetry;

use rand::Rng;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use self::command::{CommandExecutor, CommandReceiver, OperationalState};
use self::metrics::PerformanceMetrics;
use self::telemetry::TelemetryGenerator;

const COMMAND_PORT_OFFSET: u16 = 1;
const FAULT_INJECTION_INTERVAL_MS: u64 = 60000;

struct MockOCS {
    telemetry_socket: UdpSocket,
    command_socket: UdpSocket,
    target_addr: String,
    metrics: PerformanceMetrics,
    telemetry_gen: TelemetryGenerator,
    command_executor: CommandExecutor,
    state: Arc<Mutex<OperationalState>>,
}

impl MockOCS {
    fn new(target_host: &str, target_port: u16, command_port: u16) -> std::io::Result<Self> {
        let telemetry_socket = UdpSocket::bind("0.0.0.0:0")?;
        telemetry_socket.set_nonblocking(false)?;

        let command_socket = UdpSocket::bind(format!("0.0.0.0:{}", command_port))?;
        command_socket.set_nonblocking(true)?;

        let target_addr = format!("{}:{}", target_host, target_port);

        telemetry_socket.connect(&target_addr)?;

        Ok(Self {
            telemetry_socket,
            command_socket,
            target_addr,
            metrics: PerformanceMetrics::new(),
            telemetry_gen: TelemetryGenerator::new(),
            command_executor: CommandExecutor::new(),
            state: Arc::new(Mutex::new(OperationalState::new())),
        })
    }

    fn execute_commands(&mut self) {
        if let Some(record) = self.command_executor.execute_next() {
            let was_overdue = record.execution_time_us > 2000;
            self.metrics.record_command_executed(was_overdue);

            if record.command_type == "SHUTDOWN" {
                println!("[OCS] Shutdown command received, stopping telemetry...");
                std::process::exit(0);
            }
        }
    }

    fn check_automatic_fault_injection(&mut self) -> bool {
        let now = Instant::now();
        let mut state = self.state.lock().unwrap();

        if now.duration_since(state.last_fault_injection).as_millis()
            >= FAULT_INJECTION_INTERVAL_MS as u128
        {
            state.fault_mode_active = true;
            state.last_fault_injection = now;
            drop(state);

            println!("[OCS] Automatic fault injection triggered (60s interval)");
            self.metrics.record_fault_injected();
            true
        } else {
            false
        }
    }

    fn apply_fault_mode(&mut self, is_fault_mode: bool) {
        let mut state = self.state.lock().unwrap();
        state.fault_mode_active = is_fault_mode;
    }

    fn run_normal_mode(&mut self, interval_ms: u64, count: u64) -> std::io::Result<()> {
        println!("[MOCK OCS] Starting normal telemetry mode");
        println!(
            "[MOCK OCS] Target: {}, Interval: {}ms, Count: {}",
            self.target_addr, interval_ms, count
        );

        let interval = Duration::from_millis(interval_ms);
        let start_time = Instant::now();

        for i in 0..count {
            let packet_start = Instant::now();
            let timestamp_ms = start_time.elapsed().as_millis() as u64;

            let telemetry = self.telemetry_gen.generate_normal(timestamp_ms);
            let packet = telemetry.to_bytes();

            match self.telemetry_socket.send(&packet) {
                Ok(bytes_sent) => {
                    let latency = packet_start.elapsed().as_micros() as u128;
                    self.metrics.record_send(latency, bytes_sent, false);

                    if i % 10 == 0 || i == count - 1 {
                        println!(
                            "[MOCK OCS] Sent packet {} - Temp: {}°C, Battery: {}mV, Angle: {}°",
                            i, telemetry.temperature, telemetry.battery_mv, telemetry.antenna_angle
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[MOCK OCS] Send error at packet {}: {}", i, e);
                }
            }

            if i < count - 1 {
                std::thread::sleep(interval);
            }
        }

        self.metrics.report();
        Ok(())
    }

    fn run_edge_case_mode(&mut self, interval_ms: u64, count: u64) -> std::io::Result<()> {
        println!("[MOCK OCS] Starting edge case injection mode");
        println!(
            "[MOCK OCS] Target: {}, Interval: {}ms, Total edge cases: {}",
            self.target_addr, interval_ms, count
        );

        let interval = Duration::from_millis(interval_ms);
        let start_time = Instant::now();

        for i in 0..count {
            let packet_start = Instant::now();
            let timestamp_ms = start_time.elapsed().as_millis() as u64;

            let telemetry = self.telemetry_gen.generate_edge_case(timestamp_ms, i as u8);
            let packet = telemetry.to_bytes();

            match self.telemetry_socket.send(&packet) {
                Ok(bytes_sent) => {
                    let latency = packet_start.elapsed().as_micros() as u128;
                    self.metrics.record_send(latency, bytes_sent, true);

                    println!(
                        "[MOCK OCS] EDGE CASE {} - Temp: {}°C, Battery: {}mV, Angle: {}°",
                        i, telemetry.temperature, telemetry.battery_mv, telemetry.antenna_angle
                    );
                }
                Err(e) => {
                    eprintln!("[MOCK OCS] Send error at packet {}: {}", i, e);
                }
            }

            if i < count - 1 {
                std::thread::sleep(interval);
            }
        }

        self.metrics.report();
        Ok(())
    }

    fn run_mixed_mode(
        &mut self,
        interval_ms: u64,
        count: u64,
        edge_case_ratio: f64,
    ) -> std::io::Result<()> {
        println!("[MOCK OCS] Starting mixed mode (normal + edge cases)");
        println!(
            "[MOCK OCS] Target: {}, Interval: {}ms, Total: {}, Edge case ratio: {:.1}%",
            self.target_addr,
            interval_ms,
            count,
            edge_case_ratio * 100.0
        );

        let interval = Duration::from_millis(interval_ms);
        let start_time = Instant::now();

        for i in 0..count {
            let packet_start = Instant::now();
            let timestamp_ms = start_time.elapsed().as_millis() as u64;

            let is_edge_case = self.telemetry_gen.rng.gen_range(0.0..1.0) < edge_case_ratio;
            let telemetry = if is_edge_case {
                self.telemetry_gen.generate_edge_case(timestamp_ms, i as u8)
            } else {
                self.telemetry_gen.generate_normal(timestamp_ms)
            };

            let packet = telemetry.to_bytes();

            match self.telemetry_socket.send(&packet) {
                Ok(bytes_sent) => {
                    let latency = packet_start.elapsed().as_micros() as u128;
                    self.metrics.record_send(latency, bytes_sent, is_edge_case);

                    let mode = if is_edge_case { "EDGE" } else { "NORMAL" };
                    if i % 10 == 0 || is_edge_case {
                        println!(
                            "[MOCK OCS] [{}] Packet {} - Temp: {}°C, Battery: {}mV, Angle: {}°",
                            mode,
                            i,
                            telemetry.temperature,
                            telemetry.battery_mv,
                            telemetry.antenna_angle
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[MOCK OCS] Send error at packet {}: {}", i, e);
                }
            }

            if i < count - 1 {
                std::thread::sleep(interval);
            }
        }

        self.metrics.report();
        Ok(())
    }

    fn run_continuous_mode(&mut self, interval_ms: u64) -> std::io::Result<()> {
        println!("[MOCK OCS] Starting continuous telemetry mode (Ctrl+C to stop)");
        println!(
            "[MOCK OCS] Target: {}, Interval: {}ms",
            self.target_addr, interval_ms
        );

        let cmd_socket_clone = self.command_socket.try_clone()?;
        let state_clone = Arc::clone(&self.state);
        thread::spawn(move || {
            let mut receiver = CommandReceiver::new(cmd_socket_clone, state_clone);
            receiver.run();
        });

        let interval = Duration::from_millis(interval_ms);
        let start_time = Instant::now();
        let mut counter = 0u64;

        loop {
            let packet_start = Instant::now();
            let timestamp_ms = start_time.elapsed().as_millis() as u64;
            let scheduled_time = start_time + Duration::from_millis(interval_ms * counter);
            let drift_us = packet_start.duration_since(scheduled_time).as_micros() as i128;
            self.metrics.record_scheduling_drift(drift_us);

            let is_automatic_fault = self.check_automatic_fault_injection();
            let is_scheduled_edge = counter % 50 == 0 && counter > 0;
            let is_edge_case = is_automatic_fault || is_scheduled_edge;

            if is_automatic_fault {
                self.apply_fault_mode(true);
                thread::sleep(Duration::from_millis(100));
                self.apply_fault_mode(false);
            }

            self.execute_commands();

            let telemetry = if is_edge_case && !is_automatic_fault {
                self.telemetry_gen
                    .generate_edge_case(timestamp_ms, (counter % 6) as u8)
            } else if is_automatic_fault {
                self.telemetry_gen.generate_edge_case(timestamp_ms, 0)
            } else {
                self.telemetry_gen.generate_normal(timestamp_ms)
            };

            let packet = telemetry.to_bytes();

            match self.telemetry_socket.send(&packet) {
                Ok(bytes_sent) => {
                    let latency = packet_start.elapsed().as_micros() as u128;
                    self.metrics.record_send(latency, bytes_sent, is_edge_case);

                    if counter % 100 == 0 {
                        let mode = if is_edge_case { "EDGE" } else { "NORMAL" };
                        let fault_tag = if is_automatic_fault {
                            " [AUTO-FAULT]"
                        } else {
                            ""
                        };
                        println!(
                            "[MOCK OCS] [{}] Packet {} - Temp: {}°C, Battery: {}mV, Angle: {}°{}",
                            mode,
                            counter,
                            telemetry.temperature,
                            telemetry.battery_mv,
                            telemetry.antenna_angle,
                            fault_tag
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[MOCK OCS] Send error at packet {}: {}", counter, e);
                }
            }

            counter += 1;
            std::thread::sleep(interval);
        }
    }
}

fn print_usage(program: &str) {
    println!("Usage: {} <host> <port> [mode] [args]", program);
    println!();
    println!("Modes:");
    println!(
        "  normal <count> <interval_ms>  - Normal telemetry (default: count=100, interval=1000)"
    );
    println!("  edge <count> <interval_ms>    - Edge case injection only");
    println!("  mixed <count> <interval_ms> <ratio> - Mixed normal and edge cases");
    println!("  continuous <interval_ms>       - Continuous mode (Ctrl+C to stop)");
    println!();
    println!("Examples:");
    println!(
        "  {} localhost 8080                     # Normal mode, 100 packets, 1s interval",
        program
    );
    println!(
        "  {} localhost 8080 normal 50 500      # Normal mode, 50 packets, 500ms interval",
        program
    );
    println!(
        "  {} localhost 8080 edge 20 1000       # Edge case mode, 20 packets, 1s interval",
        program
    );
    println!(
        "  {} localhost 8080 mixed 100 500 0.1  # Mixed mode, 10% edge cases",
        program
    );
    println!(
        "  {} localhost 8080 continuous 1000    # Continuous mode, 1s interval",
        program
    );
}

pub fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        print_usage(&args[0]);
        return Ok(());
    }

    let host = &args[1];
    let port: u16 = match args[2].parse() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Invalid port: {}", args[2]);
            return Ok(());
        }
    };

    let command_port = port + COMMAND_PORT_OFFSET;
    let mode = args.get(3).map(|s| s.as_str()).unwrap_or("normal");
    let mut ocs = MockOCS::new(host, port, command_port)?;

    println!(
        "[OCS] Telemetry port: {}, Command port: {}",
        port, command_port
    );

    match mode {
        "normal" => {
            let count: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(100);
            let interval_ms: u64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(1000);
            ocs.run_normal_mode(interval_ms, count)?;
        }
        "edge" => {
            let count: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(20);
            let interval_ms: u64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(1000);
            ocs.run_edge_case_mode(interval_ms, count)?;
        }
        "mixed" => {
            let count: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(100);
            let interval_ms: u64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(1000);
            let ratio: f64 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.1);
            ocs.run_mixed_mode(interval_ms, count, ratio)?;
        }
        "continuous" => {
            let interval_ms: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1000);
            ocs.run_continuous_mode(interval_ms)?;
        }
        _ => {
            eprintln!("Unknown mode: {}", mode);
            print_usage(&args[0]);
        }
    }

    Ok(())
}

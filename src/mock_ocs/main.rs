use rand::Rng;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

const TELEMETRY_SIZE: usize = 14;

#[derive(Debug, Clone)]
struct Telemetry {
    timestamp_ms: u64,
    temperature: i16,
    battery_mv: u16,
    antenna_angle: i16,
}

impl Telemetry {
    fn to_bytes(&self) -> [u8; TELEMETRY_SIZE] {
        let mut bytes = [0u8; TELEMETRY_SIZE];
        bytes[0..8].copy_from_slice(&self.timestamp_ms.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.temperature.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.battery_mv.to_le_bytes());
        bytes[12..14].copy_from_slice(&self.antenna_angle.to_le_bytes());
        bytes
    }

    fn from_bytes(data: &[u8]) -> Self {
        let timestamp_ms = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let temperature = i16::from_le_bytes(data[8..10].try_into().unwrap());
        let battery_mv = u16::from_le_bytes(data[10..12].try_into().unwrap());
        let antenna_angle = i16::from_le_bytes(data[12..14].try_into().unwrap());
        Self {
            timestamp_ms,
            temperature,
            battery_mv,
            antenna_angle,
        }
    }
}

struct PerformanceMetrics {
    packets_sent: u64,
    total_bytes_sent: u64,
    send_latency_us: u128,
    min_latency_us: u128,
    max_latency_us: u128,
    edge_case_count: u64,
    start_time: Instant,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            packets_sent: 0,
            total_bytes_sent: 0,
            send_latency_us: 0,
            min_latency_us: u128::MAX,
            max_latency_us: 0,
            edge_case_count: 0,
            start_time: Instant::now(),
        }
    }

    fn record_send(&mut self, latency_us: u128, bytes: usize, is_edge_case: bool) {
        self.packets_sent += 1;
        self.total_bytes_sent += bytes as u64;
        self.send_latency_us += latency_us;
        self.min_latency_us = self.min_latency_us.min(latency_us);
        self.max_latency_us = self.max_latency_us.max(latency_us);
        if is_edge_case {
            self.edge_case_count += 1;
        }
    }

    fn report(&self) {
        let elapsed = self.start_time.elapsed();
        let avg_latency = if self.packets_sent > 0 {
            self.send_latency_us / self.packets_sent as u128
        } else {
            0
        };

        println!("\n=== MOCK OCS Performance Report ===");
        println!("Duration: {:?}", elapsed);
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
        println!("================================\n");
    }
}

struct MockOCS {
    socket: UdpSocket,
    target_addr: String,
    metrics: PerformanceMetrics,
    rng: rand::rngs::ThreadRng,
    base_temperature: i16,
    base_battery: u16,
}

impl MockOCS {
    fn new(target_host: &str, target_port: u16) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(false)?;

        let target_addr = format!("{}:{}", target_host, target_port);

        socket.connect(&target_addr)?;

        Ok(Self {
            socket,
            target_addr,
            metrics: PerformanceMetrics::new(),
            rng: rand::thread_rng(),
            base_temperature: 20,
            base_battery: 8000,
        })
    }

    fn generate_normal_telemetry(&mut self, timestamp_ms: u64) -> Telemetry {
        let temp_variation: i16 = self.rng.gen_range(-10..=10);
        let battery_drain: u16 = self.rng.gen_range(1..=5);
        let antenna_variation: i16 = self.rng.gen_range(-5..=5);

        Telemetry {
            timestamp_ms,
            temperature: self.base_temperature + temp_variation,
            battery_mv: self.base_battery.saturating_sub(battery_drain),
            antenna_angle: antenna_variation,
        }
    }

    fn generate_edge_case(&mut self, timestamp_ms: u64, case_type: u8) -> Telemetry {
        let telemetry = match case_type % 6 {
            0 => Telemetry {
                timestamp_ms,
                temperature: -50, // Extreme cold
                battery_mv: self.base_battery,
                antenna_angle: 0,
            },
            1 => Telemetry {
                timestamp_ms,
                temperature: 125, // Extreme heat (beyond safe limits)
                battery_mv: self.base_battery,
                antenna_angle: 0,
            },
            2 => Telemetry {
                timestamp_ms,
                temperature: self.base_temperature,
                battery_mv: 2000, // Low battery
                antenna_angle: 0,
            },
            3 => Telemetry {
                timestamp_ms,
                temperature: self.base_temperature,
                battery_mv: 0, // Critical battery
                antenna_angle: 0,
            },
            4 => Telemetry {
                timestamp_ms,
                temperature: self.base_temperature,
                battery_mv: self.base_battery,
                antenna_angle: -90, // Extreme antenna angle
            },
            _ => Telemetry {
                timestamp_ms,
                temperature: self.base_temperature,
                battery_mv: self.base_battery,
                antenna_angle: 90, // Extreme antenna angle
            },
        };

        self.metrics.record_send(0, 0, true);
        telemetry
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

            let telemetry = self.generate_normal_telemetry(timestamp_ms);
            let packet = telemetry.to_bytes();

            match self.socket.send(&packet) {
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

            let telemetry = self.generate_edge_case(timestamp_ms, i as u8);
            let packet = telemetry.to_bytes();

            match self.socket.send(&packet) {
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

            let is_edge_case = self.rng.gen_range(0.0..1.0) < edge_case_ratio;
            let telemetry = if is_edge_case {
                self.generate_edge_case(timestamp_ms, i as u8)
            } else {
                self.generate_normal_telemetry(timestamp_ms)
            };

            let packet = telemetry.to_bytes();

            match self.socket.send(&packet) {
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

        let interval = Duration::from_millis(interval_ms);
        let start_time = Instant::now();
        let mut counter = 0u64;

        loop {
            let packet_start = Instant::now();
            let timestamp_ms = start_time.elapsed().as_millis() as u64;

            let is_edge_case = counter % 50 == 0 && counter > 0;
            let telemetry = if is_edge_case {
                self.generate_edge_case(timestamp_ms, (counter % 6) as u8)
            } else {
                self.generate_normal_telemetry(timestamp_ms)
            };

            let packet = telemetry.to_bytes();

            match self.socket.send(&packet) {
                Ok(bytes_sent) => {
                    let latency = packet_start.elapsed().as_micros() as u128;
                    self.metrics.record_send(latency, bytes_sent, is_edge_case);

                    if counter % 100 == 0 {
                        let mode = if is_edge_case { "EDGE" } else { "NORMAL" };
                        println!(
                            "[MOCK OCS] [{}] Packet {} - Temp: {}°C, Battery: {}mV, Angle: {}°",
                            mode,
                            counter,
                            telemetry.temperature,
                            telemetry.battery_mv,
                            telemetry.antenna_angle
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

fn main() -> std::io::Result<()> {
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

    let mode = args.get(3).map(|s| s.as_str()).unwrap_or("normal");
    let mut ocs = MockOCS::new(host, port)?;

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

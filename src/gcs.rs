use std::collections::VecDeque;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

pub const TELEMETRY_SIZE: usize = 14;
const LOSS_OF_CONTACT_THRESHOLD: u32 = 3;
const EXPECTED_PACKET_INTERVAL_MS: u64 = 500;
const DECODE_LATENCY_THRESHOLD_US: u128 = 3000;
const COMMAND_DISPATCH_THRESHOLD_US: u128 = 2000;
const FAULT_RESPONSE_THRESHOLD_MS: u64 = 100;

#[derive(Debug, Clone)]
pub struct Telemetry {
    pub timestamp_ms: u64,
    pub temperature: i16,
    pub battery_mv: u16,
    pub antenna_angle: i16,
}

impl Telemetry {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < TELEMETRY_SIZE {
            return None;
        }

        Some(Self {
            timestamp_ms: u64::from_le_bytes(data[0..8].try_into().ok()?),
            temperature: i16::from_le_bytes(data[8..10].try_into().ok()?),
            battery_mv: u16::from_le_bytes(data[10..12].try_into().ok()?),
            antenna_angle: i16::from_le_bytes(data[12..14].try_into().ok()?),
        })
    }

    pub fn is_critical(&self) -> bool {
        self.temperature > 100 || self.temperature < -40 || self.battery_mv < 3000
    }

    pub fn is_edge_case(&self) -> bool {
        self.temperature < -40
            || self.temperature > 100
            || self.battery_mv < 3000
            || self.antenna_angle.abs() > 45
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    pub command_id: u32,
    pub command_type: String,
    pub priority: u8,
    pub timestamp: Instant,
    pub deadline: Duration,
}

impl Command {
    pub fn new(command_id: u32, command_type: &str, priority: u8, max_delay: Duration) -> Self {
        Self {
            command_id,
            command_type: command_type.to_string(),
            priority,
            timestamp: Instant::now(),
            deadline: max_delay,
        }
    }

    pub fn is_overdue(&self) -> bool {
        self.timestamp.elapsed() > self.deadline
    }
}

#[derive(Debug)]
pub enum Fault {
    HighTemperature(i16),
    LowBattery(u16),
    AntennaMisalignment(i16),
    PacketLoss(u32),
    LossOfContact,
}

pub struct GCSPerformanceMetrics {
    packets_received: u64,
    valid_packets: u64,
    invalid_packets: u64,
    edge_cases_detected: u64,
    critical_events: u64,
    total_bytes_received: u64,
    decode_latency_us: u128,
    min_decode_us: u128,
    max_decode_us: u128,
    packets_lost: u32,
    consecutive_lost: u32,
    loss_of_contact_count: u32,
    commands_received: u64,
    commands_dispatched: u64,
    commands_overdue: u64,
    commands_rejected: u64,
    faults_detected: u64,
    fault_response_times_ms: Vec<u128>,
    interlock_count: u64,
    start_time: Instant,
    expected_packet_times: VecDeque<Instant>,
    packet_backlog: usize,
    jitter_us: Vec<u128>,
    last_packet_time: Option<Instant>,
}

impl GCSPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            packets_received: 0,
            valid_packets: 0,
            invalid_packets: 0,
            edge_cases_detected: 0,
            critical_events: 0,
            total_bytes_received: 0,
            decode_latency_us: 0,
            min_decode_us: u128::MAX,
            max_decode_us: 0,
            packets_lost: 0,
            consecutive_lost: 0,
            loss_of_contact_count: 0,
            commands_received: 0,
            commands_dispatched: 0,
            commands_overdue: 0,
            commands_rejected: 0,
            faults_detected: 0,
            fault_response_times_ms: Vec::new(),
            interlock_count: 0,
            start_time: Instant::now(),
            expected_packet_times: VecDeque::new(),
            packet_backlog: 0,
            jitter_us: Vec::new(),
            last_packet_time: None,
        }
    }

    pub fn record_packet_received(
        &mut self,
        bytes: usize,
        decode_time_us: u128,
        is_valid: bool,
        is_edge_case: bool,
        is_critical: bool,
    ) {
        self.packets_received += 1;
        self.total_bytes_received += bytes as u64;

        if let Some(last_time) = self.last_packet_time {
            let interval_us = last_time.elapsed().as_micros() as u128;
            let expected_us: u128 = EXPECTED_PACKET_INTERVAL_MS as u128 * 1000;
            let jitter = if interval_us > expected_us {
                interval_us - expected_us
            } else {
                expected_us - interval_us
            };
            self.jitter_us.push(jitter);
        }
        self.last_packet_time = Some(Instant::now());

        if is_valid {
            self.valid_packets += 1;
            self.decode_latency_us += decode_time_us;
            self.min_decode_us = self.min_decode_us.min(decode_time_us);
            self.max_decode_us = self.max_decode_us.max(decode_time_us);

            if decode_time_us > DECODE_LATENCY_THRESHOLD_US {
                println!(
                    "[GCS-WARN] Decode latency {}μs exceeds 3ms threshold!",
                    decode_time_us
                );
            }
        } else {
            self.invalid_packets += 1;
            self.packets_lost += 1;
            self.consecutive_lost += 1;
        }

        if is_edge_case {
            self.edge_cases_detected += 1;
        }

        if is_critical {
            self.critical_events += 1;
        }
    }

    pub fn record_packet_lost(&mut self) {
        self.packets_lost += 1;
        self.consecutive_lost += 1;

        if self.consecutive_lost >= LOSS_OF_CONTACT_THRESHOLD {
            self.loss_of_contact_count += 1;
            println!(
                "[GCS-ALERT] LOSS OF CONTACT! {} consecutive packets missed",
                self.consecutive_lost
            );
        }
    }

    pub fn record_packet_ack(&mut self) {
        self.consecutive_lost = 0;
    }

    pub fn record_command_received(&mut self) {
        self.commands_received += 1;
    }

    pub fn record_command_dispatched(&mut self, dispatch_time_us: u128, was_overdue: bool) {
        self.commands_dispatched += 1;
        if was_overdue {
            self.commands_overdue += 1;
            println!(
                "[GCS-WARN] Command dispatched {}μs overdue",
                dispatch_time_us
            );
        }

        if dispatch_time_us > COMMAND_DISPATCH_THRESHOLD_US {
            println!(
                "[GCS-WARN] Command dispatch {}μs exceeds 2ms threshold!",
                dispatch_time_us
            );
        }
    }

    pub fn record_command_rejected(&mut self, reason: &str) {
        self.commands_rejected += 1;
        println!("[GCS-REJECT] Command rejected: {}", reason);
    }

    pub fn record_fault(&mut self, fault: &Fault) {
        self.faults_detected += 1;
        match fault {
            Fault::HighTemperature(temp) => {
                println!("[GCS-FAULT] High temperature detected: {}°C", temp)
            }
            Fault::LowBattery(mv) => println!("[GCS-FAULT] Low battery: {}mV", mv),
            Fault::AntennaMisalignment(angle) => {
                println!("[GCS-FAULT] Antenna misalignment: {}°", angle)
            }
            Fault::PacketLoss(count) => println!("[GCS-FAULT] Packet loss: {} packets", count),
            Fault::LossOfContact => println!("[GCS-FAULT] Loss of contact with satellite"),
        }
    }

    pub fn record_fault_response(&mut self, response_time_ms: u128) {
        self.fault_response_times_ms.push(response_time_ms);
        if response_time_ms > FAULT_RESPONSE_THRESHOLD_MS as u128 {
            println!(
                "[GCS-CRITICAL] Fault response {}ms exceeds 100ms threshold!",
                response_time_ms
            );
        }
    }

    pub fn record_interlock(&mut self, reason: &str) {
        self.interlock_count += 1;
        println!("[GCS-INTERLOCK] Safety interlock triggered: {}", reason);
    }

    pub fn record_re_request(&mut self, packet_id: u64) {
        println!(
            "[GCS-RE-REQUEST] Requesting retransmission of packet #{}",
            packet_id
        );
    }

    pub fn report(&self) {
        let elapsed = self.start_time.elapsed();
        let avg_decode = if self.valid_packets > 0 {
            self.decode_latency_us / self.valid_packets as u128
        } else {
            0
        };

        let avg_jitter = if !self.jitter_us.is_empty() {
            self.jitter_us.iter().sum::<u128>() / self.jitter_us.len() as u128
        } else {
            0
        };

        let avg_fault_response = if !self.fault_response_times_ms.is_empty() {
            self.fault_response_times_ms.iter().sum::<u128>()
                / self.fault_response_times_ms.len() as u128
        } else {
            0
        };

        println!("\n{}", "=".repeat(60));
        println!("GCS PERFORMANCE REPORT");
        println!("{}", "=".repeat(60));
        println!("Duration: {:?}", elapsed);
        println!("\n--- Telemetry Reception ---");
        println!("Packets received: {}", self.packets_received);
        println!("Valid packets: {}", self.valid_packets);
        println!("Invalid packets: {}", self.invalid_packets);
        println!("Packets lost: {}", self.packets_lost);
        println!("Consecutive lost (max): {}", self.consecutive_lost);
        println!("Loss of contact events: {}", self.loss_of_contact_count);
        println!("Edge cases detected: {}", self.edge_cases_detected);
        println!("Critical events: {}", self.critical_events);
        println!("Total bytes received: {}", self.total_bytes_received);
        println!(
            "Packets/second: {:.2}",
            self.packets_received as f64 / elapsed.as_secs_f64()
        );
        println!("\n--- Latency & Jitter ---");
        println!(
            "Average decode latency: {} μs (target: <3000μs)",
            avg_decode
        );
        println!("Min decode latency: {} μs", self.min_decode_us);
        println!("Max decode latency: {} μs", self.max_decode_us);
        println!("Average jitter: {} μs", avg_jitter);
        println!("\n--- Command Uplink ---");
        println!("Commands received: {}", self.commands_received);
        println!("Commands dispatched: {}", self.commands_dispatched);
        println!("Commands overdue: {}", self.commands_overdue);
        println!("Commands rejected: {}", self.commands_rejected);
        println!("\n--- Fault Management ---");
        println!("Faults detected: {}", self.faults_detected);
        println!("Interlock activations: {}", self.interlock_count);
        println!(
            "Average fault response: {} ms (target: <100ms)",
            avg_fault_response
        );
        let realtime_status = if self.max_decode_us < DECODE_LATENCY_THRESHOLD_US {
            "All real-time constraints MET"
        } else {
            "!!! DECODE LATENCY EXCEEDS 3ms !!!"
        };
        println!("{}", "=".repeat(60));
        println!("{}", realtime_status);
        println!("{}", "=".repeat(60));
    }
}

pub struct GCS {
    socket: UdpSocket,
    metrics: GCSPerformanceMetrics,
    pending_rerequests: Vec<u64>,
    command_queue: Vec<Command>,
    fault_active: bool,
}

impl GCS {
    pub fn new(port: u16) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        socket.set_nonblocking(false)?;
        Ok(Self {
            socket,
            metrics: GCSPerformanceMetrics::new(),
            pending_rerequests: Vec::new(),
            command_queue: Vec::new(),
            fault_active: false,
        })
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        println!("[GCS] Ground Control Station started - Port {}", {
            let addr = self.socket.local_addr().unwrap();
            addr.port()
        });
        println!(
            "[GCS] Loss of contact threshold: {} packets",
            LOSS_OF_CONTACT_THRESHOLD
        );
        println!("[GCS] Decode latency target: <3ms");
        println!("[GCS] Command dispatch target: <2ms");
        println!("[GCS] Fault response target: <100ms");

        let mut buffer = [0u8; TELEMETRY_SIZE];
        let mut _packet_counter = 0u64;

        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((bytes_read, sender_addr)) => {
                    let decode_start = Instant::now();
                    _packet_counter += 1;
                    self.metrics.record_packet_ack();

                    if let Some(telemetry) = Telemetry::from_bytes(&buffer[..bytes_read]) {
                        let decode_time = decode_start.elapsed().as_micros() as u128;
                        let is_edge = telemetry.is_edge_case();
                        let is_critical = telemetry.is_critical();

                        self.metrics.record_packet_received(
                            bytes_read,
                            decode_time,
                            true,
                            is_edge,
                            is_critical,
                        );

                        let edge_tag = if is_edge { " [EDGE]" } else { "" };
                        let critical_tag = if is_critical { " [CRITICAL]" } else { "" };
                        println!(
                            "[GCS]{}{} #{} - Temp: {}°C, Battery: {}mV, Angle: {}°, Decode: {}μs{}",
                            edge_tag,
                            critical_tag,
                            self.metrics.packets_received,
                            telemetry.temperature,
                            telemetry.battery_mv,
                            telemetry.antenna_angle,
                            decode_time,
                            if decode_time > 3000 {
                                " [LATENCY VIOLATION]"
                            } else {
                                ""
                            }
                        );

                        if is_critical {
                            self.metrics
                                .record_fault(&Fault::HighTemperature(telemetry.temperature));
                        }

                        if self.metrics.packets_received % 50 == 0 {
                            self.metrics.report();
                        }
                    } else {
                        self.metrics
                            .record_packet_received(bytes_read, 0, false, false, false);
                        println!(
                            "[GCS] Invalid packet from {}: {} bytes",
                            sender_addr, bytes_read
                        );
                        self.metrics.record_fault(&Fault::PacketLoss(1));
                    }
                }
                Err(e) => {
                    self.metrics.record_packet_lost();
                    if self.metrics.consecutive_lost == LOSS_OF_CONTACT_THRESHOLD {
                        self.metrics.record_fault(&Fault::LossOfContact);
                    }
                    eprintln!("[GCS] Receive error: {}", e);
                }
            }
        }
    }
}

pub fn run_gcs(port: u16) -> std::io::Result<()> {
    let mut gcs = GCS::new(port)?;
    gcs.run()
}

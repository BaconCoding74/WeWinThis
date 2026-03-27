use crate::{
    logger::{Logger, log_command, log_critical_alert, now_ms},
    network::{MessageType, Packet, encode_packet},
    system_state::{RuntimeMode, SystemState},
};
use std::{
    collections::VecDeque,
    io::Write,
    net::TcpStream,
    sync::atomic::{AtomicU32, Ordering},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum CommandType {
    SetNormal = 1,
    SetDegraded = 2,
    Ping = 3,
}

#[derive(Debug, Copy, Clone)]
pub struct Command {
    pub cmd_type: CommandType,
    pub arg: i16,
}

#[derive(Debug, Copy, Clone)]
pub struct ScheduledCommand {
    pub command: Command,
    pub scheduled_time: Instant,
    pub deadline: Duration,
}

pub fn send_command(
    command: &ScheduledCommand,
    stream: &mut TcpStream,
    gcs_stats: Option<&Arc<crate::GcsStats>>,
) {
    static SEQ: AtomicU32 = AtomicU32::new(1);

    let sequence = SEQ.fetch_add(1, Ordering::Relaxed);
    let timestamp = now_ms() as u32;

    let mut payload = [0u8; 14];

    payload[0] = command.command.cmd_type as u8;
    payload[1..3].copy_from_slice(&command.command.arg.to_le_bytes());

    payload[3] = 0;

    let payload_len = 4;

    let packet = Packet {
        msg_type: MessageType::Command,
        seq: sequence,
        timestamp_ms: timestamp,
        payload_len,
        payload,
    };

    let bytes = encode_packet(&packet);

    if let Err(e) = stream.write_all(&bytes) {
        eprintln!("Failed to send command: {}", e);
        return;
    }

    if let Some(stats) = gcs_stats {
        stats.record_command_sent();
    }

    println!(
        "Command sent: {:?}, seq: {}",
        command.command.cmd_type, sequence
    );
}

pub struct CommandScheduler {
    pub queue: VecDeque<ScheduledCommand>,
    pub stream: TcpStream,
    pub fault_logger: Option<Arc<Mutex<Logger>>>,
    pub gcs_stats: Option<Arc<crate::GcsStats>>,
}

impl CommandScheduler {
    pub fn new(
        stream: TcpStream,
        fault_logger: Option<Arc<Mutex<Logger>>>,
        gcs_stats: Option<Arc<crate::GcsStats>>,
    ) -> Self {
        Self {
            queue: VecDeque::new(),
            stream,
            fault_logger,
            gcs_stats,
        }
    }

    pub fn schedule(&mut self, cmd: ScheduledCommand) {
        self.queue.push_back(cmd);
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn run(&mut self, system_state: &SystemState, logger: &mut Logger) -> bool {
        let now = Instant::now();

        let should_run = self
            .queue
            .front()
            .is_some_and(|front| now >= front.scheduled_time);

        if should_run {
            let cmd = self.queue.pop_front().unwrap();

            let dispatch_latency = now.duration_since(cmd.scheduled_time);
            let latency_ms = dispatch_latency.as_millis();
            let cmd_type = format!("{:?}", cmd.command.cmd_type);

            let ts = now_ms();

            if !self.validate_command(&cmd.command, system_state) {
                println!("Command rejected by safety interlock");

                let interlock_latency_ms = if system_state.has_active_fault() {
                    let fault_time = system_state.get_fault_detect_time();
                    if fault_time > 0 {
                        (ts as i128 - fault_time as i128).max(0) as u128
                    } else {
                        0
                    }
                } else {
                    0
                };

                println!("[INTERLOCK] Fault detected {}ms ago", interlock_latency_ms);

                if interlock_latency_ms > 100 {
                    eprintln!(
                        "[CRITICAL ALERT] Fault response time {}ms exceeds 100ms threshold!",
                        interlock_latency_ms
                    );

                    if let Some(ref fault_log) = self.fault_logger {
                        let mut fl = fault_log.lock().unwrap();
                        log_critical_alert(
                            &mut fl,
                            ts,
                            "FAULT_RESPONSE_TIMEOUT",
                            interlock_latency_ms,
                            "Ground alert triggered",
                        );
                    }
                }

                log_command(logger, ts, &cmd_type, latency_ms, "REJECTED_INTERLOCK");

                if let Some(ref stats) = self.gcs_stats {
                    stats.record_command_interlock();
                }

                return true;
            }

            if dispatch_latency > cmd.deadline {
                println!("Deadline missed: {:?}", dispatch_latency);
                log_command(logger, ts, &cmd_type, latency_ms, "MISSED_DEADLINE");
            } else {
                println!("Command dispatched within deadline");
                log_command(logger, ts, &cmd_type, latency_ms, "ON_TIME");
            }

            send_command(&cmd, &mut self.stream, self.gcs_stats.as_ref());

            return true;
        }

        false
    }

    fn validate_command(&self, command: &Command, state: &SystemState) -> bool {
        let mode = state.get_mode();
        let temp = state.get_temperature();
        let overheated = state.is_overheated();

        match command.cmd_type {
            CommandType::SetNormal => mode != RuntimeMode::Safe && !overheated && temp < 80.0,
            CommandType::SetDegraded => true,
            CommandType::Ping => true,
        }
    }
}

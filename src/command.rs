use crate::{
    logger::{log_command, now_ms, Logger},
    network::{encode_packet, MessageType, Packet},
    system_state::{RuntimeMode, SystemState},
};
use std::{
    collections::VecDeque,
    io::Write,
    net::TcpStream,
    sync::atomic::{AtomicU32, Ordering},
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

pub fn send_command(command: &ScheduledCommand, stream: &mut TcpStream) {
    static SEQ: AtomicU32 = AtomicU32::new(1);

    let sequence = SEQ.fetch_add(1, Ordering::Relaxed);
    let timestamp = now_ms() as u32;

    let mut payload = [0u8; 14];

    payload[0] = command.command.cmd_type as u8;
    payload[1..3].copy_from_slice(&command.command.arg.to_le_bytes());

    // flags (not used yet)
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

    println!(
        "Command sent: {:?}, seq: {}",
        command.command.cmd_type, sequence
    );
}

pub struct CommandScheduler {
    pub queue: VecDeque<ScheduledCommand>,
    pub stream: TcpStream,
}

impl CommandScheduler {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            queue: VecDeque::new(),
            stream,
        }
    }

    pub fn schedule(&mut self, cmd: ScheduledCommand) {
        self.queue.push_back(cmd);
    }

    pub fn run(&mut self, system_state: &SystemState, logger: &mut Logger) -> bool {
        let now = Instant::now();

        if let Some(front) = self.queue.front()
        && now >= front.scheduled_time
    {
        let cmd = self.queue.pop_front().unwrap();

        let dispatch_latency = now.duration_since(cmd.scheduled_time);
        let latency_ms = dispatch_latency.as_millis();
        let cmd_type = format!("{:?}", cmd.command.cmd_type);

        let ts = now_ms();

        if !self.validate_command(&cmd.command, system_state) {
            println!("Command rejected by safety interlock");

            log_command(logger, ts, &cmd_type, latency_ms, "REJECTED_INTERLOCK");
            return true;
        }

        if dispatch_latency > cmd.deadline {
            println!("Deadline missed: {:?}", dispatch_latency);
            log_command(logger, ts, &cmd_type, latency_ms, "MISSED_DEADLINE");
        } else {
            println!("Command dispatched within deadline");
            log_command(logger, ts, &cmd_type, latency_ms, "ON_TIME");
        }

        send_command(&cmd, &mut self.stream);

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

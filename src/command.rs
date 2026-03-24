use std::{
    collections::VecDeque,
    fs,
    io::Write,
    net::TcpStream,
    time::{Duration, Instant},
};

use serde::Deserialize;

use crate::{
    constants::PACKET_SIZE,
    logger::{log_command, now_ms, Logger},
    network::{encode_packet, MessageType, Packet},
    system_state::{SystemMode, SystemState},
};

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum SensorType {
    Thermal = 1,
    Camera = 2,
    Payload = 3,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum CommandType {
    SetSensorRate = 1,
    ResetSensor = 2,
    RequestTelemetry = 3,
    RequestSafeMode = 4,
}

#[derive(Debug, Copy, Clone)]
pub struct Command {
    pub cmd_type: CommandType,
    pub sensor: Option<SensorType>,
    pub value: Option<u32>,
}

#[derive(Debug, Copy, Clone)]
pub struct ScheduledCommand {
    pub command: Command,
    pub scheduled_time: Instant,
    pub deadline: Duration,
}

pub fn send_command(command: &ScheduledCommand, stream: &mut TcpStream) {
    let sequence = now_ms() as u32;

    let mut payload = [0u8; 14];
    payload[0] = command.command.cmd_type as u8;
    payload[1] = match command.command.sensor {
        Some(sensor) => sensor as u8,
        None => 0,
    };

    if let Some(value) = command.command.value {
        payload[2..6].copy_from_slice(&value.to_be_bytes());
    }

    let packet = Packet {
        msg_type: MessageType::Command,
        seq: sequence,
        payload,
    };

    let bytes = encode_packet(&packet);
    debug_assert_eq!(bytes.len(), PACKET_SIZE);

    if let Err(e) = stream.write_all(&bytes) {
        eprintln!("Failed to send command: {}", e);
        return;
    }

    println!("Command packet sent: {:?}", command.command.cmd_type);
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

            if !self.validate_command(&cmd.command, system_state) {
                println!("Command rejected by safety interlock");

                log_command(logger, now_ms(), &cmd_type, latency_ms, "REJECTED");
                return true;
            }

            if dispatch_latency > cmd.deadline {
                println!("Deadline missed: {:?}", dispatch_latency);
                log_command(logger, now_ms(), &cmd_type, latency_ms, "MISSED_DEADLINE");
            } else {
                println!("Command dispatched within deadline");
                log_command(logger, now_ms(), &cmd_type, latency_ms, "ON_TIME");
            }

            send_command(&cmd, &mut self.stream);
            return true;
        }

        false
    }

    fn validate_command(&self, command: &Command, state: &SystemState) -> bool {
        match state.system_mode {
            SystemMode::Emergency => {
                matches!(command.cmd_type, CommandType::RequestSafeMode)
            }
            SystemMode::Safe => true,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CommandConfig {
    pub cmd_type: String,
    pub sensor: Option<String>,
    pub value: Option<u32>,
    pub delay_ms: u64,
}

pub fn load_schedule(path: &str) -> Vec<CommandConfig> {
    let data = fs::read_to_string(path).expect("Failed to read preset_schedule.json");
    serde_json::from_str(&data).expect("Failed to parse JSON schedule")
}

pub fn parse_sensor_type(s: &str) -> SensorType {
    match s {
        "Thermal" => SensorType::Thermal,
        "Camera" => SensorType::Camera,
        "Payload" => SensorType::Payload,
        _ => panic!("Unknown sensor type"),
    }
}

pub fn parse_command_type(s: &str) -> CommandType {
    match s {
        "RequestTelemetry" => CommandType::RequestTelemetry,
        "SetSensorRate" => CommandType::SetSensorRate,
        "ResetSensor" => CommandType::ResetSensor,
        "RequestSafeMode" => CommandType::RequestSafeMode,
        _ => panic!("Unknown command type"),
    }
}

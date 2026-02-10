use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::Instant;

pub const MAX_COMMAND_QUEUE_SIZE: usize = 20;

#[derive(Debug, Clone)]
pub struct Command {
    pub command_id: u32,
    pub command_type: String,
    pub priority: u8,
    pub timestamp: Instant,
    pub payload: String,
}

impl Command {
    pub fn new(command_id: u32, command_type: &str, priority: u8, payload: &str) -> Self {
        Self {
            command_id,
            command_type: command_type.to_string(),
            priority,
            timestamp: Instant::now(),
            payload: payload.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct ExecutionRecord {
    pub command_id: u32,
    pub command_type: String,
    pub executed_at: Instant,
    pub execution_time_us: u128,
    pub result: String,
}

pub struct CommandExecutor {
    queue: Vec<Command>,
    execution_history: Vec<ExecutionRecord>,
    next_command_id: u32,
}

impl CommandExecutor {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            execution_history: Vec::new(),
            next_command_id: 1,
        }
    }

    pub fn add_command(&mut self, command: Command) {
        if self.queue.len() >= MAX_COMMAND_QUEUE_SIZE {
            println!("[OCS-CMD] Command queue full, dropping oldest command");
            self.queue.remove(0);
        }
        let cmd_clone = command.clone();
        self.queue.push(command);
        self.queue.sort_by_key(|c| std::cmp::Reverse(c.priority));
        println!(
            "[OCS-CMD] Queued command #{}: {} (priority: {})",
            cmd_clone.command_id, cmd_clone.command_type, cmd_clone.priority
        );
    }

    pub fn execute_next(&mut self) -> Option<ExecutionRecord> {
        if self.queue.is_empty() {
            return None;
        }

        let command = self.queue.remove(0);
        let start = Instant::now();
        let result = format!("Executed command #{}", command.command_id);
        let exec_time = start.elapsed().as_micros() as u128;

        let record = ExecutionRecord {
            command_id: command.command_id,
            command_type: command.command_type.clone(),
            executed_at: start,
            execution_time_us: exec_time,
            result: result.clone(),
        };

        self.execution_history.push(record.clone());
        println!(
            "[OCS-CMD] Executed {} in {}μs - {}",
            command.command_type, exec_time, result
        );

        Some(record)
    }

    pub fn get_next_id(&mut self) -> u32 {
        let id = self.next_command_id;
        self.next_command_id += 1;
        id
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }
}

#[derive(Clone)]
pub struct OperationalState {
    pub mode: String,
    pub fault_mode_active: bool,
    pub consecutive_missed_thermal: u32,
    pub safety_alert_active: bool,
    pub last_fault_injection: Instant,
}

impl OperationalState {
    pub fn new() -> Self {
        Self {
            mode: "normal".to_string(),
            fault_mode_active: false,
            consecutive_missed_thermal: 0,
            safety_alert_active: false,
            last_fault_injection: Instant::now(),
        }
    }
}

pub struct CommandReceiver {
    socket: UdpSocket,
    state: Arc<Mutex<OperationalState>>,
}

impl CommandReceiver {
    pub fn new(socket: UdpSocket, state: Arc<Mutex<OperationalState>>) -> Self {
        Self { socket, state }
    }

    pub fn run(&mut self) {
        let mut buffer = [0u8; 256];
        println!("[OCS] Command receiver started");

        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((bytes_read, _sender_addr)) => {
                    let cmd_str = String::from_utf8_lossy(&buffer[..bytes_read]);
                    println!("[OCS] Received command: {}", cmd_str.trim());
                    self.process_command(&cmd_str);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("[OCS] Command receive error: {}", e);
                }
            }
        }
    }

    fn process_command(&mut self, cmd_str: &str) {
        let parts: Vec<&str> = cmd_str.trim().split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let response = match parts[0].to_uppercase().as_str() {
            "INJECT_FAULT" => {
                let fault_type = parts.get(1).map(|s| *s).unwrap_or("random");
                self.handle_inject_fault(fault_type)
            }
            "SET_MODE" => {
                let mode = parts.get(1).map(|s| *s).unwrap_or("normal");
                self.handle_set_mode(mode)
            }
            "GET_STATUS" => self.handle_get_status(),
            "SHUTDOWN" => {
                println!("[OCS] Shutdown command received");
                std::process::exit(0);
            }
            "PING" => "[OCS] PONG".to_string(),
            _ => format!("[OCS] Unknown command: {}", parts[0]),
        };

        println!("{}", response);
    }

    fn handle_inject_fault(&mut self, fault_type: &str) -> String {
        let mut state = self.state.lock().unwrap();
        state.fault_mode_active = true;
        drop(state);

        std::thread::sleep(Duration::from_millis(100));

        let mut state = self.state.lock().unwrap();
        state.fault_mode_active = false;
        let recovery_start = Instant::now();

        let fault_msg = match fault_type {
            "temp" => "High temperature fault injected".to_string(),
            "battery" => "Low battery fault injected".to_string(),
            "antenna" => "Antenna misalignment fault injected".to_string(),
            "random" => "Random fault injected".to_string(),
            _ => format!(
                "Fault type '{}' not recognized, injecting random",
                fault_type
            ),
        };

        std::thread::sleep(Duration::from_millis(10));
        let recovery_time = recovery_start.elapsed().as_millis();

        format!("[OCS] {} - Recovery time: {}ms", fault_msg, recovery_time)
    }

    fn handle_set_mode(&mut self, mode: &str) -> String {
        let mut state = self.state.lock().unwrap();
        state.mode = mode.to_string();
        format!("[OCS] Mode set to: {}", mode)
    }

    fn handle_get_status(&self) -> String {
        let state = self.state.lock().unwrap();
        format!(
            "[OCS] Status - Mode: {}, Fault Active: {}, Safety Alert: {}",
            state.mode, state.fault_mode_active, state.safety_alert_active
        )
    }
}

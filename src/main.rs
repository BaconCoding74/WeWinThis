mod command;
mod command_schedule;
mod constants;
mod gcs;
mod logger;
mod network;
mod system_state;
mod telemetry;
mod thermal;

use std::{sync::Arc, sync::atomic::AtomicU32, thread, time::Duration};

use crate::{
    gcs::{create_shared_loggers, tcp_listener},
    system_state::SystemState,
};

fn main() {
    let system_state = Arc::new(SystemState::new());

    let (command_logger, telemetry_logger, system_state_logger, performance_logger, fault_logger) =
        create_shared_loggers();

    let telemetry_backlog = Arc::new(AtomicU32::new(0));

    let state_clone = Arc::clone(&system_state);
    let command_logger_clone = Arc::clone(&command_logger);
    let telemetry_logger_clone = Arc::clone(&telemetry_logger);
    let system_state_logger_clone = Arc::clone(&system_state_logger);
    let fault_logger_clone = Arc::clone(&fault_logger);
    let performance_logger_clone = Arc::clone(&performance_logger);
    let backlog_clone = Arc::clone(&telemetry_backlog);

    thread::spawn(move || {
        tcp_listener(
            state_clone,
            command_logger_clone,
            telemetry_logger_clone,
            system_state_logger_clone,
            fault_logger_clone,
            performance_logger_clone,
            backlog_clone,
        );
    });

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

mod command;
mod constants;
mod gcs;
mod logger;
mod network;
mod system_state;
mod telemetry;

use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{
    gcs::{create_shared_loggers, gcs_scheduler, tcp_listener},
    system_state::SystemState,
};

fn main() {
    let system_state = Arc::new(Mutex::new(SystemState::new()));

    let (command_logger, telemetry_logger, system_state_logger, performance_logger) =
        create_shared_loggers();

    let state_receiver = Arc::clone(&system_state);
    let telemetry_logger_clone = Arc::clone(&telemetry_logger);
    let system_state_logger_clone = Arc::clone(&system_state_logger);

    thread::spawn(move || {
        tcp_listener(
            state_receiver,
            telemetry_logger_clone,
            system_state_logger_clone,
        );
    });

    let state_scheduler = Arc::clone(&system_state);
    let command_logger_clone = Arc::clone(&command_logger);
    let performance_logger_clone = Arc::clone(&performance_logger);

    thread::spawn(move || {
        gcs_scheduler(
            state_scheduler,
            command_logger_clone,
            performance_logger_clone,
        );
    });

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

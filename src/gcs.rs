use std::{
    fs::{self, File},
    io::Read,
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::{
    command::{
        load_schedule, parse_command_type, parse_sensor_type, Command, CommandScheduler,
        ScheduledCommand,
    },
    constants::{ASSET_DIR, LOCALHOST, LOG_DIR, PACKET_SIZE, RECEIVER_PORT},
    logger::{log_jitter, log_system_state, log_telemetry, now_ms, Logger},
    network::{decode_packet, MessageType},
    system_state::{decode_system_state, SystemState},
    telemetry::decode_telemetry,
};

pub fn tcp_listener(
    system_state: Arc<Mutex<SystemState>>,
    telemetry_logger: Arc<Mutex<Logger>>,
    system_state_logger: Arc<Mutex<Logger>>,
) {
    let listener =
        TcpListener::bind((LOCALHOST, RECEIVER_PORT)).expect("Failed to bind TCP listener");

    println!("TCP Listening on {}:{}", LOCALHOST, RECEIVER_PORT);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New client connected");

                let state = Arc::clone(&system_state);
                let telemetry_logger_clone = Arc::clone(&telemetry_logger);
                let system_state_logger_clone = Arc::clone(&system_state_logger);

                thread::spawn(move || {
                    gcs_receiver(
                        stream,
                        state,
                        telemetry_logger_clone,
                        system_state_logger_clone,
                    );
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
}

pub fn gcs_receiver(
    mut stream: TcpStream,
    system_state: Arc<Mutex<SystemState>>,
    telemetry_logger: Arc<Mutex<Logger>>,
    system_state_logger: Arc<Mutex<Logger>>,
) {
    let mut buffer = [0u8; PACKET_SIZE];
    let mut expected_seq: Option<u32> = None;

    loop {
        match stream.read_exact(&mut buffer) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Read error / client disconnected: {}", e);
                break;
            }
        }

        let packet = match decode_packet(&buffer) {
            Ok(p) => p,
            Err(e) => {
                println!("Decode error: {}", e);
                continue;
            }
        };

        if let Some(prev) = expected_seq {
            let wanted = prev.wrapping_add(1);
            if packet.seq != wanted {
                eprintln!(
                    "Sequence gap detected: expected {}, got {}",
                    wanted, packet.seq
                );
            }
        }
        expected_seq = Some(packet.seq);

        match packet.msg_type {
            MessageType::Telemetry => {
                let Ok(t) = decode_telemetry(&packet.payload) else {
                    eprintln!("Telemetry decode failed");
                    continue;
                };

                println!(
                    "[Telemetry] seq={} temp={:.2} voltage={:.2}",
                    t.sequence, t.temperature, t.voltage
                );

                let timestamp = now_ms();

                {
                    let mut logger = telemetry_logger.lock().unwrap();
                    log_telemetry(
                        &mut logger,
                        timestamp,
                        t.sequence,
                        t.temperature,
                        t.voltage,
                        0,
                        "OK",
                    );
                }

                {
                    let mut state = system_state.lock().unwrap();
                    state.set_sequence(t.sequence);
                    state.set_temperature(t.temperature);
                }
            }

            MessageType::SystemState => {
                let Ok(t) = decode_system_state(&packet.payload) else {
                    eprintln!("System state decode failed");
                    continue;
                };

                {
                    let mut state = system_state.lock().unwrap();
                    state.set_sequence(t.last_sequence);
                    state.set_mode(t.system_mode);
                    state.set_temperature(t.last_temperature);
                }

                let timestamp = now_ms();

                {
                    let mut logger = system_state_logger.lock().unwrap();
                    log_system_state(
                        &mut logger,
                        timestamp,
                        &format!("{:?}", t.system_mode),
                        "STATE_UPDATE",
                    );
                }
            }

            MessageType::Command => {
                println!("Command packet received");
            }
        }
    }
}

pub fn gcs_scheduler(
    system_state: Arc<Mutex<SystemState>>,
    command_logger: Arc<Mutex<Logger>>,
    performance_logger: Arc<Mutex<Logger>>,
) {
    let stream =
        TcpStream::connect((LOCALHOST, RECEIVER_PORT)).expect("Failed to connect to TCP server");

    let mut scheduler = CommandScheduler::new(stream);
    let start_time = Instant::now();
    let now = Instant::now();

    let mut schedule_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    schedule_path.push(ASSET_DIR);
    schedule_path.push("preset_schedule.json");

    let configs = load_schedule(schedule_path.to_str().unwrap());

    for cfg in configs {
        let command = Command {
            cmd_type: parse_command_type(&cfg.cmd_type),
            sensor: cfg.sensor.map(|s| parse_sensor_type(&s)),
            value: cfg.value,
        };

        scheduler.schedule(ScheduledCommand {
            command,
            scheduled_time: now + Duration::from_millis(cfg.delay_ms),
            deadline: Duration::from_millis(2),
        });
    }

    let mut last_run = Instant::now();
    let mut iteration: u128 = 0;

    loop {
        let now = Instant::now();

        let actual_interval = now.duration_since(last_run);
        last_run = now;

        let expected_interval_ms = 1u128;
        let actual_interval_ms = actual_interval.as_millis();
        let jitter_ms = actual_interval_ms.abs_diff(expected_interval_ms);

        iteration += 1;
        let expected_elapsed_ms = iteration * expected_interval_ms;
        let actual_elapsed_ms = start_time.elapsed().as_millis();
        let _drift_ms = actual_elapsed_ms.abs_diff(expected_elapsed_ms);

        let state_snapshot = {
            let state = system_state.lock().unwrap();
            state.clone()
        };

        let did_run = {
            let mut cmd_logger = command_logger.lock().unwrap();
            scheduler.run(&state_snapshot, &mut cmd_logger)
        };

        if did_run {
            let mut perf_logger = performance_logger.lock().unwrap();
            log_jitter(
                &mut perf_logger,
                now_ms(),
                "scheduler_loop",
                expected_interval_ms,
                actual_interval_ms,
                jitter_ms,
            );
        }

        thread::sleep(Duration::from_millis(1));
    }
}

pub fn create_shared_loggers() -> (
    Arc<Mutex<Logger>>,
    Arc<Mutex<Logger>>,
    Arc<Mutex<Logger>>,
    Arc<Mutex<Logger>>,
) {
    let mut log_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    log_dir.push(LOG_DIR);
    fs::create_dir_all(&log_dir).expect("Failed to create logs directory");

    let mut cmd_log_path = log_dir.clone();
    cmd_log_path.push("command_log.csv");

    let mut telemetry_log_path = log_dir.clone();
    telemetry_log_path.push("telemetry_log.csv");

    let mut system_state_log_path = log_dir.clone();
    system_state_log_path.push("system_state_log.csv");

    let mut perf_log_path = log_dir;
    perf_log_path.push("performance_log.csv");

    let command_logger = Arc::new(Mutex::new(Logger::new(
        File::create(cmd_log_path).expect("Failed to create command log"),
    )));

    let telemetry_logger = Arc::new(Mutex::new(Logger::new(
        File::create(telemetry_log_path).expect("Failed to create telemetry log"),
    )));

    let system_state_logger = Arc::new(Mutex::new(Logger::new(
        File::create(system_state_log_path).expect("Failed to create system state log"),
    )));

    let performance_logger = Arc::new(Mutex::new(Logger::new(
        File::create(perf_log_path).expect("Failed to create performance log"),
    )));

    (
        command_logger,
        telemetry_logger,
        system_state_logger,
        performance_logger,
    )
}

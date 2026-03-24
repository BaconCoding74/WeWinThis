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
    command::CommandScheduler,
    command_schedule::build_command_schedule,
    constants::{LOCALHOST, LOG_DIR, PACKET_SIZE, RECEIVER_PORT},
    logger::{log_jitter, log_system_state, log_telemetry, now_ms, Logger},
    network::{decode_ack, decode_packet, AckResult, MessageType},
    system_state::SystemState,
    thermal::{
        decode_alert, decode_status, decode_thermal_packet, update_system_state_from_thermal,
        ThermalToComm,
    },
};

pub fn tcp_listener(
    system_state: Arc<SystemState>,
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
    system_state: Arc<SystemState>,
    telemetry_logger: Arc<Mutex<Logger>>,
    system_state_logger: Arc<Mutex<Logger>>,
) {
    let mut buffer = [0u8; PACKET_SIZE];
    let mut expected_seq: Option<u32> = None;

    loop {
        let n = match stream.read(&mut buffer) {
            Ok(0) => {
                eprintln!("Client disconnected");
                break;
            }
            Ok(n) => n,
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        };

        // ❗ OCS-style: reject partial packet
        if n < PACKET_SIZE {
            eprintln!("Bad packet (partial read): {} bytes", n);
            continue;
        }

        let packet = match decode_packet(&buffer) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Decode error: {}", e);
                continue;
            }
        };

        // Sequence check
        if let Some(prev) = expected_seq {
            let expected = prev.wrapping_add(1);
            if packet.seq != expected {
                eprintln!(
                    "Sequence gap detected: expected {}, got {}",
                    expected, packet.seq
                );
            }
        }
        expected_seq = Some(packet.seq);

        let now = now_ms();
        let latency_ms = now.saturating_sub(packet.timestamp_ms as u128);

        match packet.msg_type {
            MessageType::Telemetry => {
                let payload = &packet.payload[..packet.payload_len as usize];

                if payload.is_empty() {
                    eprintln!("Empty telemetry payload");
                    continue;
                }

                match payload[0] {
                    3 => match decode_status(payload) {
                        Ok(ThermalToComm::Status(s)) => {
                            update_system_state_from_thermal(
                                &system_state,
                                ThermalToComm::Status(s),
                                packet.seq,
                            );

                            println!(
                                "[Status] seq={} temp={:.2}°C target={:.2}°C",
                                packet.seq,
                                s.temp_x10 as f32 / 10.0,
                                s.target_temp_x10 as f32 / 10.0
                            );

                            let mut logger = telemetry_logger.lock().unwrap();
                            log_telemetry(
                                &mut logger,
                                now_ms(),
                                packet.seq,
                                s.temp_x10 as f32 / 10.0,
                                s.target_temp_x10 as f32 / 10.0,
                                0,
                                "STATUS",
                            );
                        }
                        Ok(_) => {
                            eprintln!("Unexpected telemetry variant");
                            continue;
                        }
                        Err(e) => {
                            eprintln!("Thermal status decode failed: {}", e);
                            continue;
                        }
                    },

                    1 => {
                        println!("[Gyro] seq={} received", packet.seq);
                    }

                    2 => {
                        println!("[Battery] seq={} received", packet.seq);
                    }

                    other => {
                        eprintln!("Unknown telemetry subtype: {}", other);
                        continue;
                    }
                }
            }

            MessageType::Fault => {
                let payload = &packet.payload[..packet.payload_len as usize];

                match decode_alert(payload) {
                    Ok(ThermalToComm::Alert(a)) => {
                        update_system_state_from_thermal(
                            &system_state,
                            ThermalToComm::Alert(a),
                            packet.seq,
                        );

                        println!(
                            "[Alert] seq={} {:?} temp={:.2}°C action={:?}",
                            packet.seq,
                            a.alert_code,
                            a.temp_x10 as f32 / 10.0,
                            a.action_code
                        );

                        let mut logger = telemetry_logger.lock().unwrap();
                        log_telemetry(
                            &mut logger,
                            now_ms(),
                            packet.seq,
                            a.temp_x10 as f32 / 10.0,
                            0.0,
                            0,
                            "ALERT",
                        );
                    }
                    Ok(_) => {
                        eprintln!("Unexpected fault variant");
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Thermal alert decode failed: {}", e);
                        continue;
                    }
                }
            }

            MessageType::Ack => {
                let payload = &packet.payload[..packet.payload_len as usize];

                match decode_ack(payload) {
                    Ok(AckResult::Success) => {
                        println!("[ACK] seq={} SUCCESS", packet.seq);
                    }
                    Ok(AckResult::Rejected(reason)) => {
                        println!("[ACK] seq={} REJECTED {:?}", packet.seq, reason);
                    }
                    Err(e) => {
                        eprintln!("ACK decode failed: {}", e);
                    }
                }
            }

            MessageType::Command => {
                println!("[INFO] Command packet received (unexpected at GCS)");
            }

            MessageType::CommandResponse => {
                println!("[CommandResponse] seq={}", packet.seq);
            }
        }

        let mut logger = system_state_logger.lock().unwrap();
        log_system_state(
            &mut logger,
            now,
            &format!("{:?}", system_state.get_mode()),
            if system_state.is_overheated() {
                "OVERHEAT"
            } else {
                "NORMAL"
            },
        );
    }
}

pub fn gcs_scheduler(
    system_state: Arc<SystemState>,
    command_logger: Arc<Mutex<Logger>>,
    performance_logger: Arc<Mutex<Logger>>,
) {
    let stream =
        TcpStream::connect((LOCALHOST, RECEIVER_PORT)).expect("Failed to connect to TCP server");

    let mut scheduler = CommandScheduler::new(stream);

    let start_time = Instant::now();
    let now = Instant::now();

    let scheduled_commands = build_command_schedule(now);

    for cmd in scheduled_commands {
        scheduler.schedule(cmd);
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

        let did_run = {
            let mut cmd_logger = command_logger.lock().unwrap();
            scheduler.run(&system_state, &mut cmd_logger)
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

type SharedLogger = Arc<Mutex<Logger>>;

pub fn create_shared_loggers() -> (SharedLogger, SharedLogger, SharedLogger, SharedLogger) {
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

use std::{
    fs::{self, File},
    io::Read,
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::{
    command::CommandScheduler,
    command_schedule::build_command_schedule,
    constants::{LOCALHOST, LOG_DIR, PACKET_SIZE, RECEIVER_PORT},
    logger::{
        log_decode_latency, log_fault, log_jitter, log_system_state, log_telemetry, now_ms, Logger,
    },
    network::{decode_ack, decode_packet, AckResult, MessageType},
    system_state::SystemState,
    telemetry::{decode_telemetry, TelemetryData},
    thermal::{decode_alert, update_system_state_from_thermal, ThermalToComm},
};

pub fn tcp_listener(
    system_state: Arc<SystemState>,
    telemetry_logger: Arc<Mutex<Logger>>,
    system_state_logger: Arc<Mutex<Logger>>,
    fault_logger: Arc<Mutex<Logger>>,
    performance_logger: Arc<Mutex<Logger>>,
    telemetry_backlog: Arc<AtomicU32>,
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
                let fault_logger_clone = Arc::clone(&fault_logger);
                let performance_logger_clone = Arc::clone(&performance_logger);
                let backlog_clone = Arc::clone(&telemetry_backlog);

                thread::spawn(move || {
                    gcs_receiver(
                        stream,
                        state,
                        telemetry_logger_clone,
                        system_state_logger_clone,
                        fault_logger_clone,
                        performance_logger_clone,
                        backlog_clone,
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
    fault_logger: Arc<Mutex<Logger>>,
    performance_logger: Arc<Mutex<Logger>>,
    telemetry_backlog: Arc<AtomicU32>,
) {
    let mut buffer = [0u8; PACKET_SIZE];
    let mut expected_seq: Option<u32> = None;
    let mut consecutive_misses: u32 = 0;
    let mut contact_lost = false;
    let mut last_packet_time: Option<Instant> = None;
    let expected_interval_ms: u128 = 100;
    let mut last_backlog_log = Instant::now();

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

        telemetry_backlog.fetch_add(1, Ordering::Release);

        // Sequence check - track missing packets
        if let Some(prev) = expected_seq {
            let expected = prev.wrapping_add(1);
            if packet.seq != expected {
                let missed_count = packet.seq.wrapping_sub(prev).saturating_sub(1);
                consecutive_misses += missed_count;
                eprintln!(
                    "Sequence gap detected: expected {}, got {} (missed {} packets)",
                    expected, packet.seq, missed_count
                );

                // Re-request mechanism: log the missed sequence for re-request
                // Actual re-request would need a bidirectional channel to OCS
                for i in 0..missed_count {
                    let missed_seq = expected.wrapping_add(i);
                    println!(
                        "[RE-REQUEST] Packet seq={} missed, needs retransmission",
                        missed_seq
                    );
                }
            } else {
                // Sequence is correct, reset consecutive misses
                consecutive_misses = 0;
                contact_lost = false;
            }
        }
        expected_seq = Some(packet.seq);

        // Loss of contact detection: >=3 consecutive missed packets
        if consecutive_misses >= 3 && !contact_lost {
            contact_lost = true;
            eprintln!(
                "[ALERT] LOSS OF CONTACT - {} consecutive packet failures",
                consecutive_misses
            );

            let mut logger = fault_logger.lock().unwrap();
            logger.log(&format!(
                "{},{},{}",
                now_ms(),
                "LOSS_OF_CONTACT",
                consecutive_misses
            ));
        }

        let now = now_ms();
        let latency_ms = now.saturating_sub(packet.timestamp_ms as u128);

        let reception_drift_ms = if let Some(last_time) = last_packet_time {
            let actual_interval = last_time.elapsed().as_millis();
            if actual_interval > 0 {
                let drift = actual_interval.saturating_sub(expected_interval_ms);
                if drift > 10 {
                    eprintln!(
                        "[RECEPTION DRIFT] seq={} interval={}ms expected={}ms drift={}ms",
                        packet.seq, actual_interval, expected_interval_ms, drift
                    );
                }
                drift
            } else {
                0
            }
        } else {
            0
        };

        last_packet_time = Some(Instant::now());

        if reception_drift_ms > 0 {
            let mut perf_log = performance_logger.lock().unwrap();
            log_jitter(
                &mut perf_log,
                now,
                "telemetry_reception",
                expected_interval_ms,
                last_packet_time.map_or(0, |t| t.elapsed().as_millis()),
                reception_drift_ms,
            );
        }

        match packet.msg_type {
            MessageType::Telemetry => {
                let payload = &packet.payload[..packet.payload_len as usize];

                let decode_start = Instant::now();
                let data = match decode_telemetry(payload) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Telemetry decode failed: {}", e);
                        continue;
                    }
                };
                let decode_latency_us = decode_start.elapsed().as_micros();
                let decode_latency_ms = decode_latency_us as f32 / 1000.0;

                if decode_latency_ms > 3.0 {
                    eprintln!(
                        "[WARN] Decode latency {:.3}ms exceeds 3ms threshold!",
                        decode_latency_ms
                    );
                }

                let now = now_ms();

                let data_type_str = match data {
                    TelemetryData::Gyro(_) => "GYRO",
                    TelemetryData::Battery(_) => "BATTERY",
                    TelemetryData::Thermal(_) => "THERMAL",
                };

                {
                    let mut perf_log = performance_logger.lock().unwrap();
                    log_decode_latency(&mut perf_log, now, data_type_str, decode_latency_ms, 3.0);
                }
                let mut logger = telemetry_logger.lock().unwrap();

                match data {
                    TelemetryData::Gyro(g) => {
                        println!(
                            "[Gyro] seq={} x={} y={} z={}",
                            packet.seq, g.x_mdps, g.y_mdps, g.z_mdps
                        );

                        log_telemetry(
                            &mut logger,
                            now,
                            packet.seq,
                            "GYRO",
                            g.x_mdps as f32,
                            g.y_mdps as f32,
                            g.z_mdps as f32,
                            latency_ms,
                        );
                    }

                    TelemetryData::Battery(b) => {
                        println!("[Battery] seq={} {}mV {}%", packet.seq, b.mv, b.pct);

                        log_telemetry(
                            &mut logger,
                            now,
                            packet.seq,
                            "BATTERY",
                            b.mv as f32,
                            b.ma as f32,
                            b.pct as f32,
                            latency_ms,
                        );
                    }

                    TelemetryData::Thermal(s) => {
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

                        log_telemetry(
                            &mut logger,
                            now,
                            packet.seq,
                            "THERMAL",
                            s.temp_x10 as f32 / 10.0,
                            s.target_temp_x10 as f32 / 10.0,
                            0.0,
                            latency_ms,
                        );
                    }
                }
            }

            MessageType::Fault => {
                let payload = &packet.payload[..packet.payload_len as usize];

                let msg = match decode_alert(payload) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!(
                            "Thermal alert decode failed: {} | len={} first_byte={}",
                            e,
                            payload.len(),
                            payload.first().copied().unwrap_or(0)
                        );
                        continue;
                    }
                };

                if let ThermalToComm::Alert(a) = msg {
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

                    let mut logger = fault_logger.lock().unwrap();

                    log_fault(
                        &mut logger,
                        now_ms(),
                        packet.seq,
                        &format!("{:?}", a.alert_code),
                        a.temp_x10 as f32 / 10.0,
                        &format!("{:?}", a.action_code),
                    );
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

        telemetry_backlog.fetch_sub(1, Ordering::Release);

        if last_backlog_log.elapsed().as_millis() >= 1000 {
            let backlog_count = telemetry_backlog.load(Ordering::Acquire);
            if backlog_count > 0 {
                eprintln!(
                    "[WARN] Telemetry backlog: {} packets pending",
                    backlog_count
                );
            }
            let mut perf_log = performance_logger.lock().unwrap();
            log_jitter(
                &mut perf_log,
                now_ms(),
                "telemetry_backlog",
                0,
                backlog_count as u128,
                backlog_count as u128,
            );
            last_backlog_log = Instant::now();
        }
    }
}

pub fn gcs_scheduler(
    system_state: Arc<SystemState>,
    command_logger: Arc<Mutex<Logger>>,
    performance_logger: Arc<Mutex<Logger>>,
    fault_logger: Arc<Mutex<Logger>>,
) {
    let stream =
        TcpStream::connect((LOCALHOST, RECEIVER_PORT)).expect("Failed to connect to TCP server");

    let mut scheduler = CommandScheduler::new(stream, Some(fault_logger));

    let start_time = Instant::now();
    let now = Instant::now();

    let scheduled_commands = build_command_schedule(now);

    for cmd in scheduled_commands {
        scheduler.schedule(cmd);
    }

    let mut last_run = Instant::now();
    let mut iteration: u128 = 0;
    let mut total_active_time_us: u128 = 0;
    let mut total_idle_time_us: u128 = 0;
    let mut last_cpu_log = Instant::now();

    loop {
        let loop_start = Instant::now();
        let now = loop_start;

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

        let after_work = Instant::now();
        let active_time = after_work.duration_since(loop_start).as_micros();
        total_active_time_us += active_time;

        thread::sleep(Duration::from_millis(1));

        let after_sleep = Instant::now();
        let idle_time = after_sleep.duration_since(after_work).as_micros();
        total_idle_time_us += idle_time;

        if last_cpu_log.elapsed().as_millis() >= 1000 {
            let total_time_us = total_active_time_us + total_idle_time_us;
            let cpu_utilization = if total_time_us > 0 {
                (total_active_time_us as f64 / total_time_us as f64 * 100.0) as u32
            } else {
                0
            };

            println!(
                "[CPU] Utilization: {}% (active: {}ms, idle: {}ms)",
                cpu_utilization,
                total_active_time_us / 1000,
                total_idle_time_us / 1000
            );

            let mut perf_logger = performance_logger.lock().unwrap();
            log_jitter(
                &mut perf_logger,
                now_ms(),
                "cpu_utilization",
                100,
                cpu_utilization as u128,
                0,
            );

            total_active_time_us = 0;
            total_idle_time_us = 0;
            last_cpu_log = Instant::now();
        }
    }
}

pub fn create_shared_loggers() -> (
    Arc<Mutex<Logger>>,
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

    let mut perf_log_path = log_dir.clone();
    perf_log_path.push("performance_log.csv");

    let mut fault_log_path = log_dir.clone();
    fault_log_path.push("fault_log.csv");

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

    let fault_logger = Arc::new(Mutex::new(Logger::new(
        File::create(fault_log_path).expect("Failed to create fault log"),
    )));

    (
        command_logger,
        telemetry_logger,
        system_state_logger,
        performance_logger,
        fault_logger,
    )
}

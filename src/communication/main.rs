use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};
use crate::common::communication_stats::CommunicationStats;
use crate::config::{CommLogSPSCBuffer, DownlinkSPSCBuffer, UplinkSPSCBuffer, COMM_PACKET_SIZE, ENTER_DEGRADE_PERCENTAGE, DOWNLINK_BUF_CAP, DOWNLINK_INIT_BUDGET, DOWNLINK_PREP_BUDGET};
use crate::common::system_state::{SystemMode, SystemState};
use crate::common::timing::TaskRuntime;
use crate::common::metrics::{duration_div, elapsed_ms_u32};
use crate::common::packet::{MessageType, Packet};
use crate::logging::communication::{CommLogCode, CommLogLevel, CommLogRecord};
use crate::protocol::command_packet::{build_ack, build_reject, decode_command_packet, validate_command, CommandCode};

pub fn run_tcp_comm_loop(
    downlink_q: &DownlinkSPSCBuffer,
    uplink_q: &UplinkSPSCBuffer,
    comm_log_q: &CommLogSPSCBuffer,
    comm_stats: &mut CommunicationStats,
    system_state: Arc<SystemState>,
    gcs_addr: SocketAddr,
) {
    let start_time = Instant::now();
    let mut runtime = TaskRuntime::new(start_time);

    let mut total = Duration::ZERO;
    let mut max_loop_time = Duration::ZERO;

    let mut tx_seq: u32 = 1;
    let mut stream: Option<TcpStream> = None;

    let mut window_open_since: Option<Instant> = None;
    let mut init_miss_logged = false;
    let mut downlink_late_logged = false;

    let mut rx_buf = [0u8; COMM_PACKET_SIZE];

    while !system_state.stop.load(Ordering::Acquire) {
        let loop_start = Instant::now();
        let now_ms = elapsed_ms_u32(start_time);

        let visible = system_state.visibility_open.load(Ordering::Acquire);

        if !visible {
            stream = None;
            window_open_since = None;
            init_miss_logged = false;
            downlink_late_logged = false;

            let loop_time = loop_start.elapsed();
            total += loop_time;
            max_loop_time = max_loop_time.max(loop_time);

            thread::sleep(Duration::from_micros(50));
            continue;
        }

        let window_open_at = match window_open_since {
            Some(t) => t,
            None => {
                let t = Instant::now();
                window_open_since = Some(t);
                init_miss_logged = false;
                downlink_late_logged = false;
                t
            }
        };

        /* TCP Connection */
        if stream.is_none() {
            let connect_start = Instant::now();

            match TcpStream::connect_timeout(&gcs_addr, DOWNLINK_INIT_BUDGET) {
                Ok(s) => {
                    let _ = s.set_nodelay(true);
                    let _ = s.set_nonblocking(true);
                    stream = Some(s);

                    let init_done_at = Instant::now();
                    comm_stats.record_connect_success(connect_start, init_done_at);

                    let init_latency_ms = connect_start.elapsed().as_millis() as i32;
                    let _ = comm_log_q.push(CommLogRecord {
                        level: CommLogLevel::Info,
                        timestamp_ms: now_ms,
                        code: CommLogCode::TcpConnected,
                        value: init_latency_ms,
                    });
                }
                Err(_) => {
                    if window_open_at.elapsed() >= DOWNLINK_INIT_BUDGET && !init_miss_logged {
                        init_miss_logged = true;
                        comm_stats.record_downlink_init_miss();

                        let _ = comm_log_q.push(CommLogRecord {
                            level: CommLogLevel::Warn,
                            timestamp_ms: now_ms,
                            code: CommLogCode::DownlinkInitMiss,
                            value: window_open_at.elapsed().as_millis() as i32,
                        });
                    }
                }
            }
        }

        /* Buffer fill percentage check */
        if downlink_q.len() * 100 >= DOWNLINK_BUF_CAP * ENTER_DEGRADE_PERCENTAGE as usize {
            system_state.set_mode(SystemMode::Degraded);

            let _ = comm_log_q.push(CommLogRecord {
                level: CommLogLevel::Warn,
                timestamp_ms: now_ms,
                code: CommLogCode::TxQueue80Pct,
                value: downlink_q.len() as i32,
            });
        }

        /* Uplink and Downlink */
        let mut clear_stream = false;
        if let Some(sock) = stream.as_mut() {
            let mut sent_this_cycle = 0u8;

            while sent_this_cycle < 4 {
                if !system_state.visibility_open.load(Ordering::Acquire) {
                    clear_stream = true;
                    break;
                }

                let Some(msg) = downlink_q.pop() else {
                    break;
                };

                let encoded = msg.packet.encode();

                match sock.write_all(&encoded) {
                    Ok(_) => {
                        let sent_at = Instant::now();

                        if let Some(cmd_rx_at) = msg.cmd_rx_at {
                            comm_stats.record_command_response_sent(
                                msg.enqueued_at,
                                sent_at,
                                cmd_rx_at,
                            );
                        }
                        else {
                            comm_stats.record_packet_sent(
                                msg.enqueued_at,
                                sent_at
                            );
                        }

                        let queue_latency_ms = sent_at.duration_since(msg.enqueued_at).as_millis() as i32;

                        let _ = comm_log_q.push(CommLogRecord {
                            level: CommLogLevel::Info,
                            timestamp_ms: now_ms,
                            code: CommLogCode::PacketSent,
                            value: queue_latency_ms,
                        });

                        tx_seq = tx_seq.wrapping_add(1);
                        sent_this_cycle += 1;
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(_) => {
                        clear_stream = true;

                        let _ = comm_log_q.push(CommLogRecord {
                            level: CommLogLevel::Error,
                            timestamp_ms: now_ms,
                            code: CommLogCode::SocketError,
                            value: -1,
                        });
                        break;
                    }
                }
            }

            if window_open_at.elapsed() >= DOWNLINK_PREP_BUDGET
                && sent_this_cycle == 0
                && downlink_q.len() > 0
                && !downlink_late_logged
            {
                downlink_late_logged = true;
                comm_stats.record_downlink_late(window_open_at, Instant::now());

                let _ = comm_log_q.push(CommLogRecord {
                    level: CommLogLevel::Warn,
                    timestamp_ms: now_ms,
                    code: CommLogCode::DownlinkLate,
                    value: window_open_at.elapsed().as_millis() as i32,
                });
            }

            /* Uplink */
            loop {
                if !system_state.visibility_open.load(Ordering::Acquire) {
                    clear_stream = true;
                    break;
                }

                match sock.read(&mut rx_buf) {
                    Ok(0) => {
                        stream = None;
                        break;
                    }
                    Ok(n) => {
                        if n < COMM_PACKET_SIZE {
                            comm_stats.record_bad_packet();

                            let _ = comm_log_q.push(CommLogRecord {
                                level: CommLogLevel::Warn,
                                timestamp_ms: now_ms,
                                code: CommLogCode::BadPacket,
                                value: n as i32,
                            });
                            break;
                        }

                        let pkt = match Packet::decode(rx_buf) {
                            Ok(p) => {
                                comm_stats.record_packet_received();
                                p
                            },
                            Err(_) => {
                                comm_stats.record_bad_packet();

                                let _ = comm_log_q.push(CommLogRecord {
                                    level: CommLogLevel::Warn,
                                    timestamp_ms: now_ms,
                                    code: CommLogCode::BadPacket,
                                    value: -2,
                                });
                                break;
                            }
                        };

                        let rx_latency_ms = now_ms.saturating_sub(pkt.timestamp_ms) as i32;

                        let _ = comm_log_q.push(CommLogRecord {
                            level: CommLogLevel::Info,
                            timestamp_ms: now_ms,
                            code: CommLogCode::PacketRecv,
                            value: rx_latency_ms,
                        });

                        if pkt.msg_type == MessageType::Command {
                            match decode_command_packet(pkt) {
                                Ok(cmd) => match validate_command(&cmd, &system_state) {
                                    Ok(()) => {
                                        if cmd.code != CommandCode::Ping {
                                            let _ = uplink_q.push(cmd);
                                        }

                                        let ack = build_ack(pkt.seq, now_ms);
                                        let _ = sock.write_all(&ack.encode());
                                    }
                                    Err(reason) => {
                                        comm_stats.record_command_rejected();

                                        let rej = build_reject(pkt.seq, now_ms, reason as u8);
                                        let _ = sock.write_all(&rej.encode());

                                        let _ = comm_log_q.push(CommLogRecord {
                                            level: CommLogLevel::Warn,
                                            timestamp_ms: now_ms,
                                            code: CommLogCode::CommandRejected,
                                            value: reason as i32,
                                        });
                                    }
                                },
                                Err(_) => {
                                    let _ = comm_log_q.push(CommLogRecord {
                                        level: CommLogLevel::Warn,
                                        timestamp_ms: now_ms,
                                        code: CommLogCode::BadPacket,
                                        value: -3,
                                    });
                                }
                            }
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(_) => {
                        stream = None;
                        let _ = comm_log_q.push(CommLogRecord {
                            level: CommLogLevel::Error,
                            timestamp_ms: now_ms,
                            code: CommLogCode::SocketError,
                            value: -4,
                        });
                        break;
                    }
                }
            }
        }

        if clear_stream {
            stream = None;
        }

        let loop_time = loop_start.elapsed();
        total += loop_time;
        max_loop_time = max_loop_time.max(loop_time);
    }
}
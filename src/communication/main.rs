use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};
use crate::common::communication_stats::CommunicationStats;
use crate::common::cpu_stats::CpuStats;
use crate::config::{CommLogSPSCBuffer, DownlinkSPSCBuffer, UplinkSPSCBuffer, COMM_PACKET_SIZE, ENTER_DEGRADE_PERCENTAGE, DOWNLINK_BUF_CAP, DOWNLINK_INIT_BUDGET, DOWNLINK_PREP_BUDGET, DETECTION_INTERVAL};
use crate::common::system_state::{SystemMode, SystemState};
use crate::common::metrics::{elapsed_ms_u32};
use crate::common::packet::{MessageType, Packet};
use crate::logging::communication::{CommLogCode, CommLogRecord};
use crate::logging::default::LogLevel;
use crate::protocol::command_packet::{build_ack, build_reject, decode_command_packet, validate_command, CommandCode, CommandRejectReason};
use crate::reports::communication::CommunicationReport;

#[inline]
fn push_comm_log(
    comm_log_q: &CommLogSPSCBuffer,
    comm_stats: &mut CommunicationStats,
    level: LogLevel,
    timestamp_ms: u32,
    code: CommLogCode,
    value: i32,
) {
    if comm_log_q.push(CommLogRecord {
        level,
        timestamp_ms,
        code,
        value,
    }).is_err() {
        comm_stats.record_log_dropped();
    }
}

pub fn run_tcp_comm_loop(
    downlink_q: &DownlinkSPSCBuffer,
    uplink_q: &UplinkSPSCBuffer,
    comm_log_q: &CommLogSPSCBuffer,
    system_state: Arc<SystemState>,
    gcs_addr: SocketAddr,
) -> CommunicationReport {
    let start_time = Instant::now();

    let cpu_stats = CpuStats::new();
    let mut comm_stats = CommunicationStats::new();

    let mut tx_seq: u32 = 1;
    let mut stream: Option<TcpStream> = None;

    let mut window_open_since: Option<Instant> = None;
    let mut init_miss_logged = false;
    let mut downlink_late_logged = false;
    let mut first_downlink_sent = false;

    let mut rx_buf = [0u8; COMM_PACKET_SIZE];

    while !system_state.stop.load(Ordering::Acquire) || !downlink_q.is_empty() {
        let loop_start = Instant::now();
        let mut idle_this_loop = Duration::ZERO;
        let now_ms = elapsed_ms_u32(start_time);

        let visible = system_state.visibility_open.load(Ordering::Acquire);

        if !visible {
            stream = None;
            window_open_since = None;
            init_miss_logged = false;
            downlink_late_logged = false;
            first_downlink_sent = false;

            thread::sleep(DETECTION_INTERVAL);

            idle_this_loop += DETECTION_INTERVAL;
            cpu_stats.add_idle(DETECTION_INTERVAL);

            cpu_stats.add_active(loop_start.elapsed() - idle_this_loop);
            continue;
        }

        let window_open_at = match window_open_since {
            Some(t) => t,
            None => {
                let t = Instant::now();
                window_open_since = Some(t);
                init_miss_logged = false;
                downlink_late_logged = false;
                first_downlink_sent = false;
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

                    push_comm_log(
                        comm_log_q,
                        &mut comm_stats,
                        LogLevel::Info,
                        now_ms,
                        CommLogCode::TcpConnected,
                        connect_start.elapsed().as_millis() as i32,
                    );
                }
                Err(_) => {
                    if window_open_at.elapsed() >= DOWNLINK_INIT_BUDGET && !init_miss_logged {
                        init_miss_logged = true;
                        comm_stats.record_downlink_init_miss();

                        push_comm_log(
                            comm_log_q,
                            &mut comm_stats,
                            LogLevel::Warn,
                            now_ms,
                            CommLogCode::DownlinkInitMiss,
                            window_open_at.elapsed().as_millis() as i32,
                        );
                    }
                }
            }
        }

        /* Buffer fill percentage check */
        if downlink_q.len() * 100 >= DOWNLINK_BUF_CAP * ENTER_DEGRADE_PERCENTAGE as usize {
            system_state.set_mode(SystemMode::Degraded);

            push_comm_log(
                comm_log_q,
                &mut comm_stats,
                LogLevel::Warn,
                now_ms,
                CommLogCode::TxQueue80Pct,
                downlink_q.len() as i32,
            );
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
                        let queue_latency_ms = sent_at.duration_since(msg.enqueued_at).as_millis() as i32;

                        if !first_downlink_sent {
                            first_downlink_sent = true;
                            comm_stats.record_downlink_prep(window_open_at, sent_at);

                            push_comm_log(
                                comm_log_q,
                                &mut comm_stats,
                                LogLevel::Info,
                                now_ms,
                                CommLogCode::DownlinkPrepReady,
                                sent_at.duration_since(window_open_at).as_millis() as i32,
                            );
                        }

                        if  let Some(cmd_rx_at) = msg.cmd_rx_at {
                            comm_stats.record_command_response_sent(
                                msg.enqueued_at,
                                sent_at,
                                cmd_rx_at,
                            );

                            push_comm_log(
                                comm_log_q,
                                &mut comm_stats,
                                LogLevel::Info,
                                now_ms,
                                CommLogCode::CommandResponseSent,
                                queue_latency_ms,
                            );
                        }
                        else {
                            comm_stats.record_packet_sent(
                                msg.enqueued_at,
                                sent_at,
                            );

                            push_comm_log(
                                comm_log_q,
                                &mut comm_stats,
                                LogLevel::Info,
                                now_ms,
                                CommLogCode::PacketSent,
                                queue_latency_ms,
                            );
                        }



                        tx_seq = tx_seq.wrapping_add(1);
                        sent_this_cycle += 1;
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(_) => {
                        clear_stream = true;

                        push_comm_log(
                            comm_log_q,
                            &mut comm_stats,
                            LogLevel::Error,
                            now_ms,
                            CommLogCode::SocketError,
                            -1,
                        );
                        break;
                    }
                }
            }

            if window_open_at.elapsed() >= DOWNLINK_PREP_BUDGET
                && !first_downlink_sent
                && downlink_q.len() > 0
                && !downlink_late_logged
            {
                downlink_late_logged = true;
                comm_stats.record_downlink_late(window_open_at, Instant::now());

                push_comm_log(
                    comm_log_q,
                    &mut comm_stats,
                    LogLevel::Warn,
                    now_ms,
                    CommLogCode::DownlinkLate,
                    window_open_at.elapsed().as_millis() as i32,
                );
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

                            push_comm_log(
                                comm_log_q,
                                &mut comm_stats,
                                LogLevel::Warn,
                                now_ms,
                                CommLogCode::BadPacket,
                                n as i32,
                            );
                            break;
                        }

                        let pkt = match Packet::decode(rx_buf) {
                            Ok(p) => {
                                comm_stats.record_packet_received();
                                p
                            },
                            Err(_) => {
                                comm_stats.record_bad_packet();

                                push_comm_log(
                                    comm_log_q,
                                    &mut comm_stats,
                                    LogLevel::Warn,
                                    now_ms,
                                    CommLogCode::BadPacket,
                                    -2,
                                );
                                break;
                            }
                        };

                        let rx_latency_ms = now_ms.saturating_sub(pkt.timestamp_ms) as i32;

                        push_comm_log(
                            comm_log_q,
                            &mut comm_stats,
                            LogLevel::Info,
                            now_ms,
                            CommLogCode::PacketRecv,
                            rx_latency_ms,
                        );

                        if pkt.msg_type == MessageType::Command {
                            if system_state.stop.load(Ordering::Acquire) {
                                comm_stats.record_command_rejected();

                                let rej = build_reject(pkt.seq, now_ms, CommandRejectReason::SystemStopped as u8);
                                let _ = sock.write_all(&rej.encode());

                                push_comm_log(
                                    comm_log_q,
                                    &mut comm_stats,
                                    LogLevel::Warn,
                                    now_ms,
                                    CommLogCode::CommandRejected,
                                    CommandRejectReason::SystemStopped as i32,
                                );

                                continue;
                            }
                            match decode_command_packet(pkt) {
                                Ok(cmd) => match validate_command(&cmd, &system_state) {
                                    Ok(()) => {
                                        if cmd.code == CommandCode::Ping {
                                            let ack = build_ack(pkt.seq, now_ms);
                                            let _ = sock.write_all(&ack.encode());
                                        } else {
                                            if uplink_q.push(cmd).is_ok() {
                                                push_comm_log(
                                                    comm_log_q,
                                                    &mut comm_stats,
                                                    LogLevel::Info,
                                                    now_ms,
                                                    CommLogCode::UplinkEnqueued,
                                                    0,
                                                );

                                                let ack = build_ack(pkt.seq, now_ms);
                                                let _ = sock.write_all(&ack.encode());
                                            } else {
                                                comm_stats.record_command_rejected();

                                                push_comm_log(
                                                    comm_log_q,
                                                    &mut comm_stats,
                                                    LogLevel::Warn,
                                                    now_ms,
                                                    CommLogCode::UplinkQueueFull,
                                                    0,
                                                );

                                                let rej = build_reject(pkt.seq, now_ms, CommandRejectReason::QueueFull as u8);
                                                let _ = sock.write_all(&rej.encode());
                                            }
                                        }
                                    }
                                    Err(reason) => {
                                        comm_stats.record_command_rejected();

                                        let rej = build_reject(pkt.seq, now_ms, reason as u8);
                                        let _ = sock.write_all(&rej.encode());

                                        push_comm_log(
                                            comm_log_q,
                                            &mut comm_stats,
                                            LogLevel::Warn,
                                            now_ms,
                                            CommLogCode::CommandRejected,
                                            reason as i32,
                                        );
                                    }
                                },
                                Err(_) => {
                                    push_comm_log(
                                        comm_log_q,
                                        &mut comm_stats,
                                        LogLevel::Warn,
                                        now_ms,
                                        CommLogCode::BadPacket,
                                        -3,
                                    );
                                }
                            }
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(_) => {
                        stream = None;
                        push_comm_log(
                            comm_log_q,
                            &mut comm_stats,
                            LogLevel::Error,
                            now_ms,
                            CommLogCode::SocketError,
                            -4,
                        );
                        break;
                    }
                }
            }
        }

        if clear_stream {
            stream = None;
        }

        cpu_stats.add_active(loop_start.elapsed() - idle_this_loop);
    }
    
    CommunicationReport {
        cpu_stats,
        comm_stats,
    }
}
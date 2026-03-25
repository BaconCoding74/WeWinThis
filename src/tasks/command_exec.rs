use std::sync::Arc;
use std::time::Instant;
use crate::common::command_stats::CommandStats;
use crate::common::metrics::elapsed_ms_u32;
use crate::common::packet::{MessageType, Packet};
use crate::common::system_state::{SystemMode, SystemState};
use crate::config::{CommandLogSPSCBuffer, DownlinkSPSCBuffer, SchedulerLogSPSCBuffer, UplinkSPSCBuffer, COMM_PAYLOAD_SIZE, MAX_COMMANDS_PER_RUN};
use crate::logging::command::{CommandLogCode, CommandLogRecord, CommandLogRejectReason};
use crate::logging::default::{LogLevel, LogRecord, LogSource};
use crate::protocol::command_packet::{command_response_to_packet, CommandCode};

#[inline]
fn push_cmd_log(
    cmd_log_q: &CommandLogSPSCBuffer,
    cmd_stats: &mut CommandStats,
    start_time: Instant,
    level: LogLevel,
    code: CommandLogCode,
    cmd_seq: u32,
    cmd_type: u8,
    reject_reason: CommandLogRejectReason,
    value: i32,
) {
    let record = CommandLogRecord {
        level,
        timestamp_ms: elapsed_ms_u32(start_time),
        code,
        cmd_seq,
        cmd_type,
        reject_reason,
        value,
    };
    
    if cmd_log_q.push(record).is_err() {
        cmd_stats.record_log_dropped();
    };
}

pub fn run_command_exec_job(
    uplink_q: &UplinkSPSCBuffer,
    downlink_q: &DownlinkSPSCBuffer,
    system_state: &Arc<SystemState>,
    cmd_log_q: &CommandLogSPSCBuffer,
    cmd_stats: &mut CommandStats,
    tx_seq: &mut u64,
) {
    let mut handled = 0u8;

    while handled < MAX_COMMANDS_PER_RUN {
        let Some(cmd) = uplink_q.pop() else {
            break;
        };
        cmd_stats.record_received();

        let exec_start = Instant::now();
        let mut result_code: u8 = 0;

        match cmd.code {
            CommandCode::SetNormal => {
                system_state.set_mode(SystemMode::Normal);
                push_cmd_log(
                    cmd_log_q,
                    cmd_stats,
                    exec_start,
                    LogLevel::Info,
                    CommandLogCode::ModeSetNormal,
                    cmd.seq,
                    cmd.code as u8,
                    CommandLogRejectReason::None,
                    0,
                );
            }

            CommandCode::SetDegraded => {
                system_state.set_mode(SystemMode::Degraded);
                push_cmd_log(
                    cmd_log_q,
                    cmd_stats,
                    exec_start,
                    LogLevel::Info,
                    CommandLogCode::ModeSetDegraded,
                    cmd.seq,
                    cmd.code as u8,
                    CommandLogRejectReason::None,
                    0,
                );
            }
            CommandCode::Ping => {

            }
        }

        let exec_finish = Instant::now();
        cmd_stats.record_execution(exec_start, exec_finish);

        let enqueued_at = Instant::now();
        let pkt = command_response_to_packet(cmd, enqueued_at, *tx_seq as u32, 0);

        if downlink_q.push(pkt).is_ok() {
            *tx_seq = tx_seq.wrapping_add(1);
            cmd_stats.record_response_enqueued(cmd.rx_instant, enqueued_at);
        }
        else {
            cmd_stats.record_response_drop();

            push_cmd_log(
                cmd_log_q,
                cmd_stats,
                exec_start,
                LogLevel::Error,
                CommandLogCode::ResponseDropped,
                cmd.seq,
                cmd.code as u8,
                CommandLogRejectReason::ResponseQueueFull,
                0,
            );
        }
        handled += 1;
    }
}
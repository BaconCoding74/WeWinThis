use std::time::Instant;
use crate::common::metrics::elapsed_ms_u32;
use crate::common::packet::{next_tx_seq, DownlinkItem};
use crate::common::scheduler_stats::SchedulerStats;
use crate::config::{CompressionLogSPSCBuffer, DownlinkSPSCBuffer, SensorSPSCBuffer, ThermalSPSCBuffer, DOWNLINK_RESERVED_FOR_CMD, MAX_PACKETS_PER_COMPRESSION_RUN};
use crate::logging::compression::{CompressionLogCode, CompressionLogRecord};
use crate::logging::default::LogLevel;
use crate::protocol::telemetry_packet::{sensor_to_packet, thermal_to_packet};

#[inline]
fn push_compression_log(
    compression_log_q: &CompressionLogSPSCBuffer,
    scheduler_q_stats: &mut SchedulerStats,
    start_time: Instant,
    level: LogLevel,
    code: CompressionLogCode,
    value: i32,
) {
    if compression_log_q.push(CompressionLogRecord {
        level,
        timestamp_ms: elapsed_ms_u32(start_time),
        code,
        value,
    }).is_err() {
        scheduler_q_stats.compression_log_dropped = scheduler_q_stats.compression_log_dropped.saturating_add(1);
    };
}

#[inline]
fn try_push_compressed_packet(
    downlink_q: &DownlinkSPSCBuffer,
    compression_log_q: &CompressionLogSPSCBuffer,
    scheduler_q_stats: &mut SchedulerStats,
    start_time: Instant,
    pkt: DownlinkItem,
) -> bool {
    if !compression_can_push(downlink_q) {
        push_compression_log(
            compression_log_q,
            scheduler_q_stats,
            start_time,
            LogLevel::Warn,
            CompressionLogCode::DownlinkReserveBlocked,
            downlink_q.free_len() as i32,
        );
        scheduler_q_stats.compression_packet_dropped = scheduler_q_stats.compression_packet_dropped.saturating_add(1);
        return false;
    }

    if downlink_q.push(pkt).is_err() {
        push_compression_log(
            compression_log_q,
            scheduler_q_stats,
            start_time,
            LogLevel::Error,
            CompressionLogCode::DownlinkPushFailed,
            downlink_q.free_len() as i32,
        );
        scheduler_q_stats.compression_packet_dropped = scheduler_q_stats.compression_packet_dropped.saturating_add(1);
        return false;
    }

    true
}

fn compression_can_push(downlink_q: &DownlinkSPSCBuffer) -> bool {
    downlink_q.free_len() > DOWNLINK_RESERVED_FOR_CMD
}

pub fn run_compression_job(
    sensor_q: &SensorSPSCBuffer,
    thermal_q: &ThermalSPSCBuffer,
    downlink_q: &DownlinkSPSCBuffer,
    compression_log_q: &CompressionLogSPSCBuffer,
    scheduler_q_stats: &mut SchedulerStats,
    start_time: Instant,
    tx_seq: &mut u64,
) {
    let mut packed = 0u8;

    while packed < MAX_PACKETS_PER_COMPRESSION_RUN {
        if thermal_q.is_empty() && sensor_q.is_empty() {
            break;
        }

        if let Some(msg) = thermal_q.pop() {
            let pkt = thermal_to_packet(msg, Instant::now(), next_tx_seq(tx_seq));

            if !try_push_compressed_packet(
                downlink_q,
                compression_log_q,
                scheduler_q_stats,
                start_time,
                pkt,
            ) {
                break;
            }

            scheduler_q_stats.compression_packet_count = scheduler_q_stats.compression_packet_count.saturating_add(1);
            packed += 1;

            if packed >= MAX_PACKETS_PER_COMPRESSION_RUN {
                break;
            }
        }

        if let Some(msg) = sensor_q.pop() {
            let pkt = sensor_to_packet(msg, Instant::now(), next_tx_seq(tx_seq));

            if !try_push_compressed_packet(
                downlink_q,
                compression_log_q,
                scheduler_q_stats,
                start_time,
                pkt,
            ) {
                break;
            }

            scheduler_q_stats.compression_packet_count = scheduler_q_stats.compression_packet_count.saturating_add(1);
            packed += 1;
        }
    }
}
use std::time::Instant;
use crate::common::packet::next_tx_seq;
use crate::config::{DownlinkSPSCBuffer, SensorSPSCBuffer, ThermalSPSCBuffer, DOWNLINK_RESERVED_FOR_CMD, MAX_PACKETS_PER_COMPRESSION_RUN};
use crate::protocol::telemetry_packet::{sensor_to_packet, thermal_to_packet};

fn compression_can_push(downlink_q: &DownlinkSPSCBuffer) -> bool {
    downlink_q.free_len() > DOWNLINK_RESERVED_FOR_CMD
}

pub fn run_compression_job(
    sensor_q: &SensorSPSCBuffer,
    thermal_q: &ThermalSPSCBuffer,
    downlink_q: &DownlinkSPSCBuffer,
    tx_seq: &mut u64,
) {
    let mut packed = 0u8;

    while packed < MAX_PACKETS_PER_COMPRESSION_RUN {
        if thermal_q.is_empty() && sensor_q.is_empty() {
            break;
        }

        if !compression_can_push(downlink_q) {
            break;
        }
        
        if let Some(msg) = thermal_q.pop() {
            let pkt = thermal_to_packet(msg, Instant::now(), next_tx_seq(tx_seq));
            if downlink_q.push(pkt).is_err() {
                break;
            }
            packed += 1;

            if packed >= MAX_PACKETS_PER_COMPRESSION_RUN {
                break;
            }
        }

        if !compression_can_push(downlink_q) {
            break;
        }

        if let Some(msg) = sensor_q.pop() {
            let pkt = sensor_to_packet(msg, Instant::now(), next_tx_seq(tx_seq));
            if downlink_q.push(pkt).is_err() {
                break;
            }
            packed += 1;
        }
    }
}
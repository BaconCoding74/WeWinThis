use std::time::Instant;
use crate::common::packet::{DownlinkItem, MessageType, Packet};
use crate::common::sensors::{BatteryMsg, GyroMsg, SensorData};
use crate::config::COMM_PAYLOAD_SIZE;
use crate::thermal_control::structs::{ThermalAlertMsg, ThermalStatusMsg, ThermalToComm};

pub fn sensor_to_packet(msg: SensorData, enqueued_at: Instant, seq: u32) -> DownlinkItem {
    let mut payload = [0u8; COMM_PAYLOAD_SIZE];
    let packet: Packet;

    match msg {
        SensorData::Gyro(GyroMsg {
            timestamp_ms,
            x_mdps,
            y_mdps,
            z_mdps,
                         }) => {
            payload[0] = 1;
            payload[1..5].copy_from_slice(&timestamp_ms.to_le_bytes());
            payload[5..7].copy_from_slice(&x_mdps.to_le_bytes());
            payload[7..9].copy_from_slice(&y_mdps.to_le_bytes());
            payload[9..11].copy_from_slice(&z_mdps.to_le_bytes());

            packet = Packet {
                msg_type: MessageType::Telemetry,
                seq,
                timestamp_ms,
                payload_len: 11,
                payload,
            };
        }

        SensorData::Battery(BatteryMsg {
            timestamp_ms,
            ma,
            mv,
            pct,
                            }) => {
            payload[0] = 2;
            payload[1..5].copy_from_slice(&timestamp_ms.to_le_bytes());
            payload[5..7].copy_from_slice(&ma.to_le_bytes());
            payload[7..9].copy_from_slice(&mv.to_le_bytes());
            payload[9] = pct;

            packet = Packet {
                msg_type: MessageType::Telemetry,
                seq,
                timestamp_ms,
                payload_len: 10,
                payload,
            };
        }
    }

    DownlinkItem {
        packet,
        enqueued_at,
        cmd_rx_at: None,
    }
}

pub fn thermal_to_packet(msg: ThermalToComm, enqueued_at: Instant, seq: u32) -> DownlinkItem {
    let mut payload = [0u8; COMM_PAYLOAD_SIZE];
    let packet: Packet;

    match msg {
        ThermalToComm::Status(ThermalStatusMsg {
                                  timestamp_ms,
                                  temp_x10,
                                  target_temp_x10,
                                  actuator_pct,
                                  flags,
                              }) => {
            payload[0] = 3;
            payload[1..5].copy_from_slice(&timestamp_ms.to_le_bytes());
            payload[5..7].copy_from_slice(&temp_x10.to_le_bytes());
            payload[7..9].copy_from_slice(&target_temp_x10.to_le_bytes());
            payload[9] = actuator_pct;
            payload[10] = flags;

            packet = Packet {
                msg_type: MessageType::Telemetry,
                seq,
                timestamp_ms,
                payload_len: 11,
                payload,
            };
        }

        ThermalToComm::Alert(ThermalAlertMsg {
                                 timestamp_ms,
                                 alert_code,
                                 temp_x10,
                                 flags,
                                 action_code,
                             }) => {
            payload[0] = 1;
            payload[1..5].copy_from_slice(&timestamp_ms.to_le_bytes());
            payload[5] = alert_code as u8;
            payload[6..8].copy_from_slice(&temp_x10.to_le_bytes());
            payload[8] = flags;
            payload[9] = action_code as u8;

            packet = Packet {
                msg_type: MessageType::Fault,
                seq,
                timestamp_ms,
                payload_len: 10,
                payload,
            };
        }
    }

    DownlinkItem {
        packet,
        enqueued_at,
        cmd_rx_at: None,
    }
}

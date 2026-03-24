use std::time::Instant;
use crate::common::packet::{DownlinkItem, MessageType, Packet};
use crate::common::system_state::{SystemMode, SystemState};
use crate::config::COMM_PAYLOAD_SIZE;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCode {
    SetNormal = 1,
    SetDegraded = 2,
    Ping = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRejectReason {
    Invalid = 1,
    AlreadyNormal = 2,
}


impl TryFrom<u8> for CommandCode {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(CommandCode::SetNormal),
            2 => Ok(CommandCode::SetDegraded),
            3 => Ok(CommandCode::Ping),
            _ => Err("invalid command"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UplinkCommand {
    pub rx_instant: Instant,
    pub seq: u32,
    pub code: CommandCode,
    pub arg_i16: i16,
}

pub fn decode_command_packet(pkt: Packet) -> Result<UplinkCommand, &'static str> {
    if pkt.msg_type != MessageType::Command {
        return Err("not a command packet");
    }

    if pkt.payload_len < 4 {
        return Err("command payload too short");
    }

    let code = CommandCode::try_from(pkt.payload[0])?;
    let arg_i16 = i16::from_le_bytes([pkt.payload[1], pkt.payload[2]]);
    let _flags = pkt.payload[3];

    Ok(UplinkCommand {
        rx_instant: Instant::now(),
        seq: pkt.seq,
        code,
        arg_i16,
    })
}

pub fn command_response_to_packet(
    cmd: UplinkCommand,
    enqueued_at: Instant,
    tx_seq: u32,
    result_code: u8,
) -> DownlinkItem {
    let mut payload = [0u8; COMM_PAYLOAD_SIZE];
    payload[0] = cmd.code as u8;
    payload[1] = result_code;
    payload[2..4].copy_from_slice(&cmd.arg_i16.to_le_bytes());
    payload[4..8].copy_from_slice(&cmd.seq.to_le_bytes());

    let packet = Packet {
        msg_type: MessageType::CommandResponse,
        seq: tx_seq,
        timestamp_ms: 0,
        payload_len: 8,
        payload,
    };

    DownlinkItem {
        packet,
        enqueued_at,
        cmd_rx_at: Some(cmd.rx_instant),
    }
}

pub fn build_ack(seq: u32, now_ms: u32) -> Packet {
    let mut payload = [0u8; COMM_PAYLOAD_SIZE];
    payload[0] = 1;

    Packet {
        msg_type: MessageType::Ack,
        seq,
        timestamp_ms: now_ms,
        payload_len: 1,
        payload,
    }
}

pub fn build_reject(seq: u32, now_ms: u32, reason: u8) -> Packet {
    let mut payload = [0u8; COMM_PAYLOAD_SIZE];
    payload[0] = 0;
    payload[1] = reason;

    Packet {
        msg_type: MessageType::Ack,
        seq,
        timestamp_ms: now_ms,
        payload_len: 2,
        payload,
    }
}

pub fn validate_command(cmd: &UplinkCommand, system_state: &SystemState) -> Result<(), CommandRejectReason> {
    match cmd.code {
        CommandCode::SetNormal => {
            if system_state.get_mode() == SystemMode::Normal {
                return Err(CommandRejectReason::AlreadyNormal);
            }
            Ok(())
        }
        CommandCode::SetDegraded => Ok(()),
        CommandCode::Ping => Ok(()),
        _ => Err(CommandRejectReason::Invalid),
    }
}
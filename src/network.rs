use crate::constants::{COMM_PAYLOAD_SIZE, PACKET_SIZE};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Telemetry = 1,
    Fault = 2,
    Command = 3,
    Ack = 4,
    CommandResponse = 5,
}

impl TryFrom<u8> for MessageType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::Telemetry),
            2 => Ok(MessageType::Fault),
            3 => Ok(MessageType::Command),
            4 => Ok(MessageType::Ack),
            5 => Ok(MessageType::CommandResponse),
            _ => Err("invalid message type"),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Packet {
    pub msg_type: MessageType,
    pub seq: u32,
    pub timestamp_ms: u32,
    pub payload_len: u8,
    pub payload: [u8; COMM_PAYLOAD_SIZE],
}

pub fn encode_packet(packet: &Packet) -> [u8; PACKET_SIZE] {
    let mut buf = [0u8; PACKET_SIZE];

    buf[0] = packet.msg_type as u8;

    buf[1..5].copy_from_slice(&packet.seq.to_le_bytes());
    buf[5..9].copy_from_slice(&packet.timestamp_ms.to_le_bytes());

    buf[9] = packet.payload_len;

    buf[10..10 + COMM_PAYLOAD_SIZE].copy_from_slice(&packet.payload);

    buf
}

pub fn decode_packet(buf: &[u8]) -> Result<Packet, &'static str> {
    if buf.len() != PACKET_SIZE {
        return Err("Invalid packet size");
    }

    let msg_type = MessageType::try_from(buf[0])?;

    let seq = u32::from_le_bytes(buf[1..5].try_into().map_err(|_| "Invalid seq")?);

    let timestamp_ms = u32::from_le_bytes(buf[5..9].try_into().map_err(|_| "Invalid timestamp")?);

    let payload_len = buf[9];

    if payload_len as usize > COMM_PAYLOAD_SIZE {
        return Err("Invalid payload length");
    }

    let mut payload = [0u8; COMM_PAYLOAD_SIZE];
    payload.copy_from_slice(&buf[10..10 + COMM_PAYLOAD_SIZE]);

    Ok(Packet {
        msg_type,
        seq,
        timestamp_ms,
        payload_len,
        payload,
    })
}

#[derive(Debug, Clone, Copy)]
pub enum CommandRejectReason {
    Invalid = 1,
    AlreadyNormal = 2,
}

impl TryFrom<u8> for CommandRejectReason {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(CommandRejectReason::Invalid),
            2 => Ok(CommandRejectReason::AlreadyNormal),
            _ => Err("invalid reject reason"),
        }
    }
}

pub enum AckResult {
    Success,
    Rejected(CommandRejectReason),
}

pub fn decode_ack(payload: &[u8]) -> Result<AckResult, &'static str> {
    if payload.is_empty() {
        return Err("empty ack payload");
    }

    match payload[0] {
        1 => Ok(AckResult::Success),

        0 => {
            if payload.len() < 2 {
                return Err("invalid reject payload");
            }

            let reason = CommandRejectReason::try_from(payload[1])?;
            Ok(AckResult::Rejected(reason))
        }

        _ => Err("invalid ack code"),
    }
}

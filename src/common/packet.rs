use std::time::Instant;
use crate::config::{COMM_PACKET_SIZE, COMM_PAYLOAD_SIZE};

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

#[derive(Debug)]
pub struct DownlinkItem {
    pub packet: Packet,
    pub enqueued_at: Instant,
    pub cmd_rx_at: Option<Instant>,
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

impl Packet {
    pub fn encode(&self) -> [u8; COMM_PACKET_SIZE] {
        let mut out = [0u8; COMM_PACKET_SIZE];
        out[0] = self.msg_type as u8;
        out[1..5].copy_from_slice(&self.seq.to_le_bytes());
        out[5..9].copy_from_slice(&self.timestamp_ms.to_le_bytes());
        out[9] = self.payload_len;
        out[10..10 + COMM_PAYLOAD_SIZE].copy_from_slice(&self.payload);
        out
    }

    pub fn decode(buf: [u8; COMM_PACKET_SIZE]) -> Result<Self, &'static str> {
        let msg_type = MessageType::try_from(buf[0])?;
        let seq = u32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]);
        let timestamp_ms = u32::from_le_bytes([buf[5], buf[6], buf[7], buf[8]]);
        let payload_len = buf[9];

        if payload_len as usize > COMM_PAYLOAD_SIZE {
            return Err("payload too large");
        }

        let mut payload = [0u8; COMM_PAYLOAD_SIZE];
        payload.copy_from_slice(&buf[10..10 + COMM_PAYLOAD_SIZE]);

        Ok(Self {
            msg_type,
            seq,
            timestamp_ms,
            payload_len,
            payload,
        })
    }
}

pub fn next_tx_seq(seq: &mut u64) -> u32 {
    let v = *seq as u32;
    *seq = seq.wrapping_add(1);
    v
}
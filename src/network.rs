use crate::constants::{PACKET_SIZE, PAYLOAD_SIZE};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum MessageType {
    Telemetry = 1,
    SystemState = 2,
    Command = 3,
}

impl TryFrom<u8> for MessageType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::Telemetry),
            2 => Ok(MessageType::SystemState),
            3 => Ok(MessageType::Command),
            _ => Err("Invalid message type"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Packet {
    pub msg_type: MessageType,
    pub seq: u32,
    pub payload: [u8; PAYLOAD_SIZE],
}

pub fn encode_packet(packet: &Packet) -> [u8; PACKET_SIZE] {
    let mut buf = [0u8; PACKET_SIZE];
    buf[0] = packet.msg_type as u8;
    buf[1..5].copy_from_slice(&packet.seq.to_be_bytes());
    buf[5..19].copy_from_slice(&packet.payload);
    buf
}

pub fn decode_packet(buf: &[u8]) -> Result<Packet, &'static str> {
    if buf.len() != PACKET_SIZE {
        return Err("Invalid packet size");
    }

    let msg_type = MessageType::try_from(buf[0])?;
    let seq = u32::from_be_bytes(buf[1..5].try_into().map_err(|_| "Invalid seq bytes")?);

    let mut payload = [0u8; PAYLOAD_SIZE];
    payload.copy_from_slice(&buf[5..19]);

    Ok(Packet {
        msg_type,
        seq,
        payload,
    })
}

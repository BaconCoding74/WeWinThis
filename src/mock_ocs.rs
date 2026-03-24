use std::{io::Write, net::TcpStream};

use crate::{
    constants::{LOCALHOST, RECEIVER_PORT},
    network::{encode_packet, MessageType, Packet},
    telemetry::encode_telemetry,
};

pub fn sender() {
    let mut stream =
        TcpStream::connect((LOCALHOST, RECEIVER_PORT)).expect("Failed to connect to TCP server");

    let sequence: u32 = 42;
    let temperature: f32 = 55.3;
    let voltage: f32 = 3.7;

    let payload = encode_telemetry(sequence, temperature, voltage);

    let packet = Packet {
        msg_type: MessageType::Telemetry,
        seq: sequence,
        payload,
    };

    let bytes = encode_packet(&packet);

    if let Err(e) = stream.write_all(&bytes) {
        eprintln!("Failed to send telemetry: {}", e);
        return;
    }

    println!("Telemetry packet sent (TCP)");
}

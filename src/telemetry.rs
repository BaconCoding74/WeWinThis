#[derive(Debug, Clone, Copy)]
pub struct Telemetry {
    pub sequence: u32,
    pub temperature: f32,
    pub voltage: f32,
}

pub fn encode_telemetry(sequence: u32, temperature: f32, voltage: f32) -> [u8; 14] {
    let mut payload = [0u8; 14];
    payload[0..4].copy_from_slice(&sequence.to_be_bytes());
    payload[4..8].copy_from_slice(&temperature.to_be_bytes());
    payload[8..12].copy_from_slice(&voltage.to_be_bytes());
    // payload[12..14] reserved
    payload
}

pub fn decode_telemetry(payload: &[u8; 14]) -> Result<Telemetry, &'static str> {
    let sequence = u32::from_be_bytes(
        payload[0..4]
            .try_into()
            .map_err(|_| "Invalid telemetry sequence bytes")?,
    );

    let temperature = f32::from_be_bytes(
        payload[4..8]
            .try_into()
            .map_err(|_| "Invalid telemetry temperature bytes")?,
    );

    let voltage = f32::from_be_bytes(
        payload[8..12]
            .try_into()
            .map_err(|_| "Invalid telemetry voltage bytes")?,
    );

    Ok(Telemetry {
        sequence,
        temperature,
        voltage,
    })
}

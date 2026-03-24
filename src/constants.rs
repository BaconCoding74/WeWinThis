pub const LOCALHOST: &str = "127.0.0.1";
pub const RECEIVER_PORT: u16 = 8080;

// project folders
pub const ASSET_DIR: &str = "assets";
pub const LOG_DIR: &str = "logs";

// packet layout
pub const PAYLOAD_SIZE: usize = 14;
pub const PACKET_SIZE: usize = 19; // 1(msg_type) + 4(seq) + 14(payload)

pub const LOCALHOST: &str = "127.0.0.1";
pub const RECEIVER_PORT: u16 = 8080;

// project folders
pub const LOG_DIR: &str = "logs";

// packet layout
pub const PACKET_SIZE: usize = 1 + 4 + 4 + 1 + COMM_PAYLOAD_SIZE;
pub const COMM_PAYLOAD_SIZE: usize = 14;

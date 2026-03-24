
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommLogLevel {
    Info = 0,
    Warn = 1,
    Error = 2,
    Critical = 3,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommLogCode {
    TcpConnected = 2001,
    DownlinkInitMiss = 2002,
    DownlinkLate = 2003,
    TxQueue80Pct = 2004,
    PacketSent = 2005,
    PacketRecv = 2006,
    BadPacket = 2007,
    CommandRejected = 2008,
    SocketError = 2009,
    ContactLost = 2010,
    RerequestSent = 2011,
}

#[derive(Debug, Clone, Copy)]
pub struct CommLogRecord {
    pub level: CommLogLevel,
    pub timestamp_ms: u32,
    pub code: CommLogCode,
    pub value: i32,
}
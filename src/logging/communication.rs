use crate::logging::default::LogLevel;

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
    CommandResponseSent = 2012,
    DownlinkPrepReady = 2013,
    UplinkEnqueued = 2014,
    UplinkQueueFull = 2015,
}

#[derive(Debug, Clone, Copy)]
pub struct CommLogRecord {
    pub level: LogLevel,
    pub timestamp_ms: u32,
    pub code: CommLogCode,
    pub value: i32,
}
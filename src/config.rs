use std::sync::Arc;
use std::time::Duration;
use crate::common::packet::{DownlinkItem, Packet};
use crate::common::sensors::SensorData;
use crate::logging::antenna::AntennaLogRecord;
use crate::logging::communication::CommLogRecord;
use crate::logging::default::LogRecord;
use crate::logging::health::HealthLogRecord;
use crate::logging::thermal::ThermalLogRecord;
use crate::protocol::command_packet::UplinkCommand;
use crate::queues::spsc_queue::SpscQueue;
use crate::thermal_control::structs::ThermalToComm;

// Simulation
pub const SIMULATION_TIME: u64 = 200;

// Degrade Trigger
pub const ENTER_DEGRADE_PERCENTAGE: i32 = 80;
pub const EXIT_DEGRADE_PERCENTAGE: i32 = 50;

// Capacity
pub const SENSOR_BUF_CAP: usize = 128;
pub const SCHEDULER_LOG_BUF_CAP: usize = 128;
pub const THERMAL_BUF_CAP: usize = 64;
pub const THERMAL_LOG_BUF_CAP: usize = 64;
pub const READY_QUEUE_CAP: usize = 64;
pub const UPLINK_BUF_CAP: usize = 64;
pub const DOWNLINK_BUF_CAP: usize = 128;
pub const COMM_LOG_BUF_CAP: usize = 64;
pub const ANT_LOG_BUF_CAP: usize = 64;
pub const HEALTH_LOG_BUF_CAP: usize = 64;

// Communication
pub const COMM_PAYLOAD_SIZE: usize = 14;
pub const COMM_PACKET_SIZE: usize = 1 + 4 + 4 + 1 + COMM_PAYLOAD_SIZE;
pub const DOWNLINK_INIT_BUDGET: Duration = Duration::from_millis(5);
pub const DOWNLINK_PREP_BUDGET: Duration = Duration::from_millis(30);

// Thermal Control
pub const DEFAULT_TARGET_TEMP_X10: i16 = 400;
pub const DEFAULT_MAX_TEMP_X10: i16 = 700;

// Compression
pub const MAX_PACKETS_PER_COMPRESSION_RUN: u8 = 10;

// Command Execution
pub const MAX_COMMANDS_PER_RUN: u8 = 4;
pub const DOWNLINK_RESERVED_FOR_CMD: usize = 4;

// Antenna Alignment
pub const NORMAL_STEP_DEG: i16 = 5;
pub const DEGRADED_STEP_DEG: i16 = 8;
pub const NORMAL_READY_THRESHOLD_DEG: i16 = 2;
pub const DEGRADED_READY_THRESHOLD_DEG: i16 = 5;

// Types
pub type SensorSPSCBuffer = Arc<SpscQueue<SensorData, SENSOR_BUF_CAP>>;
pub type ThermalSPSCBuffer = Arc<SpscQueue<ThermalToComm, THERMAL_BUF_CAP>>;
pub type ThermalLogSPSCBuffer = Arc<SpscQueue<ThermalLogRecord, THERMAL_LOG_BUF_CAP>>;
pub type SchedulerLogSPSCBuffer = Arc<SpscQueue<LogRecord, SCHEDULER_LOG_BUF_CAP>>;
pub type UplinkSPSCBuffer = Arc<SpscQueue<UplinkCommand, UPLINK_BUF_CAP>>;
pub type DownlinkSPSCBuffer = Arc<SpscQueue<DownlinkItem, DOWNLINK_BUF_CAP>>;
pub type CommLogSPSCBuffer = Arc<SpscQueue<CommLogRecord, COMM_LOG_BUF_CAP>>;
pub type AntLogSPSCBuffer = Arc<SpscQueue<AntennaLogRecord, ANT_LOG_BUF_CAP>>;
pub type HealthLogSPSCBuffer = Arc<SpscQueue<HealthLogRecord, HEALTH_LOG_BUF_CAP>>;

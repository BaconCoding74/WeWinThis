use std::sync::Arc;
use std::time::Duration;
use crate::common::packet::{DownlinkItem};
use crate::common::sensors::SensorData;
use crate::logging::antenna::AntennaLogRecord;
use crate::logging::command::CommandLogRecord;
use crate::logging::communication::CommLogRecord;
use crate::logging::compression::CompressionLogRecord;
use crate::logging::health::HealthLogRecord;
use crate::logging::scheduler::SchedulerLogRecord;
use crate::logging::thermal::ThermalLogRecord;
use crate::protocol::command_packet::UplinkCommand;
use crate::queues::spsc_queue::SpscQueue;
use crate::thermal_control::structs::ThermalToComm;

// Simulation
pub const SIMULATION_TIME: u64 = 250;

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
pub const COMMAND_LOG_BUF_CAP: usize = 64;
pub const COMMPRESSION_LOG_BUF_CAP: usize = 64;

// Communication
pub const COMM_PAYLOAD_SIZE: usize = 14;
pub const COMM_PACKET_SIZE: usize = 1 + 4 + 4 + 1 + COMM_PAYLOAD_SIZE;
pub const DOWNLINK_INIT_BUDGET: Duration = Duration::from_millis(5);
pub const DOWNLINK_PREP_BUDGET: Duration = Duration::from_millis(30);
pub const COMM_RESPONSE_DEADLINE: Duration = Duration::from_millis(25);
pub const DETECTION_INTERVAL: Duration = Duration::from_micros(50);

// Thermal Control
pub const DEFAULT_TARGET_TEMP_X10: i16 = 400;
pub const DEFAULT_MAX_TEMP_X10: i16 = 700;
pub const THERMAL_MISS_LIMIT: u8 = 3;
pub const THERMAL_RECOVERY_LIMIT_MS: u32 = 200;
pub const THERMAL_FAULT_PERIOD_MS: u32 = 60_000;

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

// Logger
pub const LOGGER_IDLE_SLEEP_MS: u64 = 10;
pub const LOGGER_FLUSH_INTERVAL_MS: u64 = 200;
pub const PRINT_INFO_TO_TERMINAL: bool = true;
pub const LOG_FILE_PATH: &str = "logs/rts_log.txt";

// Types
pub type SensorSPSCBuffer = Arc<SpscQueue<SensorData, SENSOR_BUF_CAP>>;
pub type ThermalSPSCBuffer = Arc<SpscQueue<ThermalToComm, THERMAL_BUF_CAP>>;
pub type ThermalLogSPSCBuffer = Arc<SpscQueue<ThermalLogRecord, THERMAL_LOG_BUF_CAP>>;
pub type SchedulerLogSPSCBuffer = Arc<SpscQueue<SchedulerLogRecord, SCHEDULER_LOG_BUF_CAP>>;
pub type UplinkSPSCBuffer = Arc<SpscQueue<UplinkCommand, UPLINK_BUF_CAP>>;
pub type DownlinkSPSCBuffer = Arc<SpscQueue<DownlinkItem, DOWNLINK_BUF_CAP>>;
pub type CommLogSPSCBuffer = Arc<SpscQueue<CommLogRecord, COMM_LOG_BUF_CAP>>;
pub type AntLogSPSCBuffer = Arc<SpscQueue<AntennaLogRecord, ANT_LOG_BUF_CAP>>;
pub type HealthLogSPSCBuffer = Arc<SpscQueue<HealthLogRecord, HEALTH_LOG_BUF_CAP>>;
pub type CommandLogSPSCBuffer = Arc<SpscQueue<CommandLogRecord, COMMAND_LOG_BUF_CAP>>;
pub type CompressionLogSPSCBuffer = Arc<SpscQueue<CompressionLogRecord, COMMAND_LOG_BUF_CAP>>;

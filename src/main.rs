mod scheduler;
mod config;
mod common;
mod protocol;
mod tasks;
mod thermal_control;
mod queues;
mod logging;
mod communication;
mod reports;
mod logger;

use std::net::{SocketAddr};
use std::sync::{Arc};
use std::sync::atomic::{Ordering};
use std::thread;
use std::time::{Duration};
use thread_priority::{set_current_thread_priority, ThreadPriority};
use crate::common::packet::{DownlinkItem};
use crate::common::sensors::SensorData;
use crate::common::system_state::{SystemState};
use crate::communication::main::run_tcp_comm_loop;
use crate::config::{SensorSPSCBuffer, COMMPRESSION_LOG_BUF_CAP, COMMAND_LOG_BUF_CAP, UPLINK_BUF_CAP, DOWNLINK_BUF_CAP, HEALTH_LOG_BUF_CAP, SENSOR_BUF_CAP, SCHEDULER_LOG_BUF_CAP, THERMAL_BUF_CAP, THERMAL_LOG_BUF_CAP, ThermalSPSCBuffer, DEFAULT_MAX_TEMP_X10, DEFAULT_TARGET_TEMP_X10, ThermalLogSPSCBuffer, SIMULATION_TIME, SchedulerLogSPSCBuffer, DownlinkSPSCBuffer, UplinkSPSCBuffer, CommLogSPSCBuffer, COMM_LOG_BUF_CAP, ANT_LOG_BUF_CAP, AntLogSPSCBuffer, HealthLogSPSCBuffer, CommandLogSPSCBuffer, CompressionLogSPSCBuffer};
use crate::logger::main::run_logger_main;
use crate::logging::antenna::AntennaLogRecord;
use crate::logging::command::CommandLogRecord;
use crate::logging::communication::{CommLogRecord};
use crate::logging::compression::CompressionLogRecord;
use crate::logging::health::HealthLogRecord;
use crate::logging::scheduler::SchedulerLogRecord;
use crate::logging::thermal::ThermalLogRecord;
use crate::protocol::command_packet::UplinkCommand;
use crate::queues::spsc_queue::SpscQueue;
use crate::reports::communication::{print_communication_report, CommunicationReport};
use crate::reports::logger::{print_logger_report, LoggerReport};
use crate::reports::scheduler::{print_scheduler_report, SchedulerReport};
use crate::reports::thermal::{print_thermal_report, ThermalReport};
use crate::scheduler::main::run_scheduler_loop;
use crate::thermal_control::main::run_thermal_loop;
use crate::thermal_control::structs::{ThermalConfig, ThermalToComm};

pub fn spawn_comm_thread(
    downlink_q: DownlinkSPSCBuffer,
    uplink_q: UplinkSPSCBuffer,
    comm_log_q: CommLogSPSCBuffer,
    system_state: Arc<SystemState>,
    gcs_addr: SocketAddr,
) -> thread::JoinHandle<CommunicationReport> {
    thread::Builder::new()
        .name("CommTcp".to_string())
        .spawn(move ||
            run_tcp_comm_loop(
                &downlink_q,
                &uplink_q,
                &comm_log_q,
                system_state,
                gcs_addr,
            )
        )
        .expect("failed to spawn CommTcp thread")
}

pub fn spawn_thermal_thread(
    thermal_to_comm_q: ThermalSPSCBuffer,
    thermal_log_q: ThermalLogSPSCBuffer,
    system_state: Arc<SystemState>,
    thermal_cfg: ThermalConfig,
) -> thread::JoinHandle<ThermalReport> {
    thread::Builder::new()
        .name("Thermal".to_string())
        .spawn(move || {
            let _ = set_current_thread_priority(ThreadPriority::Max);
            run_thermal_loop(
                &thermal_to_comm_q.clone(),
                &thermal_log_q.clone(),
                system_state.clone(),
                thermal_cfg,
            )
        })
        .expect("Failed to spawn Scheduler thread")
}

pub fn spawn_scheduler_thread(
    sensor_q: SensorSPSCBuffer,
    thermal_q: ThermalSPSCBuffer,
    downlink_q: DownlinkSPSCBuffer,
    uplink_q: UplinkSPSCBuffer,
    scheduler_log_q: SchedulerLogSPSCBuffer,
    command_log_q: CommandLogSPSCBuffer,
    ant_log_q: AntLogSPSCBuffer,
    health_log_q: HealthLogSPSCBuffer,
    compression_log_q: CompressionLogSPSCBuffer,
    system_state: Arc<SystemState>,
) -> thread::JoinHandle<SchedulerReport> {
    thread::Builder::new()
        .name("Scheduler".to_string())
        .spawn(move || run_scheduler_loop(
            &sensor_q.clone(),
            &thermal_q.clone(),
            &downlink_q.clone(),
            &uplink_q.clone(),
            &scheduler_log_q.clone(),
            &command_log_q.clone(),
            &ant_log_q.clone(),
            &health_log_q.clone(),
            &compression_log_q.clone(),
            system_state)
        )
        .expect("Failed to spawn Scheduler thread")
}

pub fn spawn_logger_thread(
    thermal_log_q: ThermalLogSPSCBuffer,
    comm_log_q: CommLogSPSCBuffer,
    scheduler_log_q: SchedulerLogSPSCBuffer,
    command_log_q: CommandLogSPSCBuffer,
    ant_log_q: AntLogSPSCBuffer,
    health_log_q: HealthLogSPSCBuffer,
    compression_log_q: CompressionLogSPSCBuffer,
    system_state: Arc<SystemState>
) -> thread::JoinHandle<LoggerReport> {
    thread::Builder::new()
        .name("Logger".to_string())
        .spawn(move || run_logger_main(
            thermal_log_q,
            comm_log_q,
            scheduler_log_q,
            command_log_q,
            ant_log_q,
            health_log_q,
            compression_log_q,
            system_state,
        )

        )
        .expect("Failed to spawn Logger thread")
}

fn main() {
    let system_state = Arc::new(SystemState::new());
    let gcs_addr: SocketAddr = "127.0.0.1:9000"
        .parse()
        .expect("Invalid GCS address");

    /* Data Buffer */
    let sensor_buffer = Arc::new(SpscQueue::<SensorData, SENSOR_BUF_CAP>::new());
    let thermal_to_comm_q = Arc::new(SpscQueue::<ThermalToComm, THERMAL_BUF_CAP>::new());
    let uplink_q = Arc::new(SpscQueue::<UplinkCommand, UPLINK_BUF_CAP>::new());
    let downlink_q = Arc::new(SpscQueue::<DownlinkItem, DOWNLINK_BUF_CAP>::new());

    /* Log Buffer */
    let thermal_log_q = Arc::new(SpscQueue::<ThermalLogRecord, THERMAL_LOG_BUF_CAP>::new());
    let scheduler_log_q = Arc::new(SpscQueue::<SchedulerLogRecord, SCHEDULER_LOG_BUF_CAP>::new());
    let comm_log_q = Arc::new(SpscQueue::<CommLogRecord, COMM_LOG_BUF_CAP>::new());
    let ant_log_q = Arc::new(SpscQueue::<AntennaLogRecord, ANT_LOG_BUF_CAP>::new());
    let health_log_q = Arc::new(SpscQueue::<HealthLogRecord, HEALTH_LOG_BUF_CAP>::new());
    let command_log_q = Arc::new(SpscQueue::<CommandLogRecord, COMMAND_LOG_BUF_CAP>::new());
    let compression_log_q = Arc::new(SpscQueue::<CompressionLogRecord, COMMPRESSION_LOG_BUF_CAP>::new());

    let thermal_cfg = ThermalConfig {
        target_temp_x10: DEFAULT_TARGET_TEMP_X10,
        max_temp_x10: DEFAULT_MAX_TEMP_X10,
    };

    let scheduler_thread = spawn_scheduler_thread(
        sensor_buffer.clone(),
        thermal_to_comm_q.clone(),
        downlink_q.clone(),
        uplink_q.clone(),
        scheduler_log_q.clone(),
        command_log_q.clone(),
        ant_log_q.clone(),
        health_log_q.clone(),
        compression_log_q.clone(),
        system_state.clone(),
    );
    let thermal_handle = spawn_thermal_thread(
        thermal_to_comm_q.clone(),
        thermal_log_q.clone(),
        system_state.clone(),
        thermal_cfg,
    );

    let comm_handle = spawn_comm_thread(
        downlink_q.clone(),
        uplink_q.clone(),
        comm_log_q.clone(),
        system_state.clone(),
        gcs_addr,
    );

    let logger_handle = spawn_logger_thread(
        thermal_log_q.clone(),
        comm_log_q.clone(),
        scheduler_log_q.clone(),
        command_log_q.clone(),
        ant_log_q.clone(),
        health_log_q.clone(),
        compression_log_q.clone(),
        system_state.clone(),
    );

    let ss = system_state.clone();
    {
        thread::spawn(move || {
            loop {
                ss.visibility_open.store(true, Ordering::Release);
                thread::sleep(Duration::from_millis(300));

                ss.visibility_open.store(false, Ordering::Release);
                thread::sleep(Duration::from_millis(300));
            }
        });
    }

    thread::sleep(Duration::from_secs(SIMULATION_TIME));
    system_state.stop.store(true, Ordering::Release);

    let scheduler_report = scheduler_thread.join().unwrap();
    let thermal_report = thermal_handle.join().unwrap();
    let logger_report = logger_handle.join().unwrap();
    let communication_report = comm_handle.join().unwrap();

    print_logger_report(&logger_report);
    print_communication_report(&communication_report);
    print_thermal_report(&thermal_report);
    print_scheduler_report(&scheduler_report);
}
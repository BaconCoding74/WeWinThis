mod scheduler;
mod config;
mod common;
mod protocol;
mod tasks;
mod thermal_control;
mod queues;
mod logging;
mod communication;

use std::net::{SocketAddr};
use std::sync::{Arc};
use std::sync::atomic::{Ordering};
use std::thread;
use std::time::{Duration};
use thread_priority::{set_current_thread_priority, ThreadPriority};
use crate::common::communication_stats::{print_comm_report, CommunicationStats};
use crate::common::packet::{DownlinkItem};
use crate::common::sensors::SensorData;
use crate::common::system_state::{SystemState};
use crate::communication::main::run_tcp_comm_loop;
use crate::config::{SensorSPSCBuffer, UPLINK_BUF_CAP, DOWNLINK_BUF_CAP, HEALTH_LOG_BUF_CAP, SENSOR_BUF_CAP, SCHEDULER_LOG_BUF_CAP, THERMAL_BUF_CAP, THERMAL_LOG_BUF_CAP, ThermalSPSCBuffer, DEFAULT_MAX_TEMP_X10, DEFAULT_TARGET_TEMP_X10, ThermalLogSPSCBuffer, SIMULATION_TIME, SchedulerLogSPSCBuffer, DownlinkSPSCBuffer, UplinkSPSCBuffer, CommLogSPSCBuffer, COMM_LOG_BUF_CAP, ANT_LOG_BUF_CAP, AntLogSPSCBuffer, HealthLogSPSCBuffer};
use crate::logging::antenna::AntennaLogRecord;
use crate::logging::communication::{CommLogRecord};
use crate::logging::default::{LogRecord};
use crate::logging::health::HealthLogRecord;
use crate::logging::thermal::ThermalLogRecord;
use crate::protocol::command_packet::UplinkCommand;
use crate::queues::spsc_queue::SpscQueue;
use crate::scheduler::main::run_scheduler_loop;
use crate::thermal_control::main::run_thermal_loop;
use crate::thermal_control::structs::{ThermalConfig, ThermalToComm};

pub fn spawn_comm_thread(
    downlink_q: DownlinkSPSCBuffer,
    uplink_q: UplinkSPSCBuffer,
    comm_log_q: CommLogSPSCBuffer,
    system_state: Arc<SystemState>,
    gcs_addr: SocketAddr,
) -> thread::JoinHandle<CommunicationStats> {
    thread::Builder::new()
        .name("CommTcp".to_string())
        .spawn(move || {
            let mut stats = CommunicationStats::new();

            run_tcp_comm_loop(
                &downlink_q,
                &uplink_q,
                &comm_log_q,
                &mut stats,
                system_state,
                gcs_addr,
            );

            stats
        })
        .expect("failed to spawn CommTcp thread")
}

pub fn spawn_thermal_thread(
    thermal_to_comm_q: ThermalSPSCBuffer,
    thermal_log_q: ThermalLogSPSCBuffer,
    system_state: Arc<SystemState>,
    thermal_cfg: ThermalConfig,
) -> thread::JoinHandle<()> {
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
    ant_log_q: AntLogSPSCBuffer,
    health_log_q: HealthLogSPSCBuffer,
    system_state: Arc<SystemState>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("Scheduler".to_string())
        .spawn(move || run_scheduler_loop(
            &sensor_q.clone(),
            &thermal_q.clone(),
            &downlink_q.clone(),
            &uplink_q.clone(),
            &scheduler_log_q.clone(),
            &ant_log_q.clone(),
            &health_log_q.clone(),
            system_state)
        )
        .expect("Failed to spawn Scheduler thread")
}

pub fn spawn_logger_thread(
    thermal_log_q: ThermalLogSPSCBuffer,
    command_log_q: CommLogSPSCBuffer,
    scheduler_log_q: SchedulerLogSPSCBuffer,
    ant_log_q: AntLogSPSCBuffer,
    health_log_q: HealthLogSPSCBuffer,
    system_state: Arc<SystemState>
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("Logger".to_string())
        .spawn(move || while !system_state.stop.load(Ordering::Acquire) {
            // while let Some(result) = thermal_log_q.pop() {
            //     println!("{:?}", result);
            // }

            // while let Some(result) = scheduler_log_q.pop() {
            //     println!("{:?}", result);
            // }
            //
            while let Some(result) = health_log_q.pop() {
                println!("{:?}", result);
            }

            while let Some(result) = ant_log_q.pop() {
                println!("{:?}", result);
            }

            while let Some(result) = command_log_q.pop() {
                println!("{:?}", result);
            }

            thread::sleep(Duration::from_millis(100));
        })
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
    let scheduler_log_q = Arc::new(SpscQueue::<LogRecord, SCHEDULER_LOG_BUF_CAP>::new());
    let comm_log_q = Arc::new(SpscQueue::<CommLogRecord, COMM_LOG_BUF_CAP>::new());
    let ant_log_q = Arc::new(SpscQueue::<AntennaLogRecord, ANT_LOG_BUF_CAP>::new());
    let health_log_q = Arc::new(SpscQueue::<HealthLogRecord, HEALTH_LOG_BUF_CAP>::new());

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
        ant_log_q.clone(),
        health_log_q.clone(),
        system_state.clone()
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
        ant_log_q.clone(),
        health_log_q.clone(),
        system_state.clone(),
    );

    let ss = system_state.clone();
    {
        thread::spawn(move || {
            loop {
                ss.visibility_open.store(true, Ordering::Release);
                thread::sleep(Duration::from_millis(30));

                ss.visibility_open.store(false, Ordering::Release);
                thread::sleep(Duration::from_millis(40));
            }
        });
    }

    thread::sleep(Duration::from_secs(SIMULATION_TIME));
    system_state.stop.store(true, Ordering::Release);

    scheduler_thread.join().unwrap();
    thermal_handle.join().unwrap();
    logger_handle.join().unwrap();
    let communication_stats = comm_handle.join().unwrap();

    print_comm_report("Communication", &communication_stats);
}
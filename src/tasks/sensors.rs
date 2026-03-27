use std::time::Instant;
use crate::common::metrics::elapsed_ms_u32;
use crate::common::scheduler_stats::SchedulerStats;
use crate::common::sensors::{BatteryMsg, GyroMsg, SensorData};
use crate::common::tasks::TaskId;
use crate::config::{HealthLogSPSCBuffer, SensorSPSCBuffer};
use crate::logging::default::LogLevel;
use crate::logging::health::{HealthLogCode, HealthLogRecord};

#[inline]
fn push_sensor_drop_log(
    health_log_q: &HealthLogSPSCBuffer,
    stats: &mut SchedulerStats,
    start_time: Instant,
    code: HealthLogCode,
) {
    let record = HealthLogRecord {
        level: LogLevel::Warn,
        timestamp_ms: elapsed_ms_u32(start_time),
        code,
        value: 1,
    };

    match code {
        HealthLogCode::GyroSampleDropped =>
            stats.gyro_dropped = stats.gyro_dropped.saturating_add(1),
        HealthLogCode::BatterySampleDropped =>
            stats.battery_dropped = stats.battery_dropped.saturating_add(1),
        _ => {}
    }

    if health_log_q.push(record).is_err() {
        stats.health_log_dropped = stats.health_log_dropped.saturating_add(1);
    }
}

fn read_gyro_sensor(seq: u64) -> GyroMsg {
    GyroMsg {
        timestamp_ms: elapsed_ms_u32(Instant::now()),
        x_mdps: 100 + ((seq % 7) as i16) * 2,
        y_mdps: 120 + ((seq % 5) as i16) * 3,
        z_mdps: 90 + ((seq % 3) as i16) * 4,
    }
}

fn read_battery_sensor(seq: u64) -> BatteryMsg {
    BatteryMsg {
        timestamp_ms: elapsed_ms_u32(Instant::now()),
        mv: 7400 - ((seq % 10) as u16 * 10),
        ma: 500 + ((seq % 5) as i16 * 20),
        pct: 90 - (seq % 20) as u8,
    }
}

fn push_sensor_reading(
    sensor_buffer: &SensorSPSCBuffer,
    health_log_q: &HealthLogSPSCBuffer,
    stats: &mut SchedulerStats,
    start_time: Instant,
    task_id: TaskId,
    data: SensorData,
) {

    if sensor_buffer.push(data).is_err() {
        match task_id {
            TaskId::Gyro => {
                push_sensor_drop_log(
                    health_log_q,
                    stats,
                    start_time,
                    HealthLogCode::GyroSampleDropped,
                );
            }
            TaskId::Battery => {
                push_sensor_drop_log(
                    health_log_q,
                    stats,
                    start_time,
                    HealthLogCode::BatterySampleDropped,
                );
            }
            _ => {}
        }
    }
}

pub fn run_sensor_job(
    sensor_buffer: &SensorSPSCBuffer,
    health_log_q: &HealthLogSPSCBuffer,
    stats: &mut SchedulerStats,
    start_time: Instant,
    task_id: TaskId,
    seq: &mut u64
) {
    match task_id {
        TaskId::Gyro => {
            let data = read_gyro_sensor(*seq);
            push_sensor_reading(
                sensor_buffer,
                health_log_q,
                stats,
                start_time,
                task_id,
                SensorData::Gyro(data),
            );

            *seq += 1;
        }
        TaskId::Battery => {
            let data = read_battery_sensor(*seq);
            push_sensor_reading(
                sensor_buffer,
                health_log_q,
                stats,
                start_time,
                task_id,
                SensorData::Battery(data),
            );

            *seq += 1;
        }
        _ => {}
    }
}
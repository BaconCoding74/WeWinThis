use std::time::Instant;
use crate::common::metrics::elapsed_ms_u32;
use crate::common::sensors::{BatteryMsg, GyroMsg, SensorData, SensorType};
use crate::common::tasks::TaskId;
use crate::config::SensorSPSCBuffer;

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
    data: SensorData,
) {
    let _ = sensor_buffer.push(data);
}

pub fn run_sensor_job(sensor_buffer: &SensorSPSCBuffer, task_id: TaskId, seq: &mut u64) {
    match task_id {
        TaskId::Gyro => {
            let data = read_gyro_sensor(*seq);
            push_sensor_reading(sensor_buffer, SensorData::Gyro(data));
        }
        TaskId::Battery => {
            let data = read_battery_sensor(*seq);
            push_sensor_reading(sensor_buffer, SensorData::Battery(data));
        }
        _ => {}
    }
}
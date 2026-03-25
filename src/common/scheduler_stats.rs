#[derive(Default)]
pub struct SchedulerStats {
    pub job_dropped: u64,
    pub gyro_dropped: u64,
    pub battery_dropped: u64,

    pub scheduler_log_dropped: u64,
    pub health_log_dropped: u64,
    pub antenna_log_dropped: u64,

    pub compression_packet_count: u64,
    pub compression_packet_dropped: u64,
    pub compression_log_dropped: u64,
}

pub fn print_queue_stats(name: &str, stats: &SchedulerStats) {
    println!("{name} Queue Stats");
    println!("  job dropped                 : {}", stats.job_dropped);
    println!("  gyro dropped                : {}", stats.gyro_dropped);
    println!("  battery dropped             : {}", stats.battery_dropped);
    println!("  scheduler log dropped       : {}", stats.scheduler_log_dropped);
    println!("  health log dropped          : {}", stats.health_log_dropped);
    println!("  antenna log dropped         : {}", stats.antenna_log_dropped);
    println!("  compression packet count    : {}", stats.compression_packet_count);
    println!("  compression packet dropped  : {}", stats.compression_packet_dropped);
    println!("  compression log dropped     : {}", stats.compression_log_dropped);
    println!();
}
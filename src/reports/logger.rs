use crate::common::cpu_stats::{print_cpu_stats, CpuStats};

pub struct LoggerReport {
    pub cpu_stats: CpuStats,
}

pub fn print_logger_report(report: &LoggerReport) {
    println!("================ Logger Report ================");
    print_cpu_stats(&report.cpu_stats);
    println!("==================================================");
}
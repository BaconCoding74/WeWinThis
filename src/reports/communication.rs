use crate::common::communication_stats::{print_comm_report, CommunicationStats};
use crate::common::cpu_stats::{print_cpu_stats, CpuStats};

pub struct CommunicationReport {
    pub comm_stats: CommunicationStats,
    pub cpu_stats: CpuStats,
}

pub fn print_communication_report(report: &CommunicationReport) {
    println!("================ Communication Report ================");
    print_cpu_stats(&report.cpu_stats);
    print_comm_report(&report.comm_stats);
    println!("==================================================");
}
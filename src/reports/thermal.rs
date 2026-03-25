use crate::common::cpu_stats::{print_cpu_stats, CpuStats};
use crate::common::metrics::print_task_report;
use crate::common::timing::{TaskRuntime, TaskTiming};

pub struct ThermalReport {
    pub runtime: TaskRuntime,
    pub cpu_stats: CpuStats,
    pub timing: TaskTiming,
}

pub fn print_thermal_report(report: &ThermalReport) {
    println!("================ Thermal Thread Report ================");
    println!("thermal runs   : {}", report.runtime.seq);
    print_cpu_stats(&report.cpu_stats);
    print_task_report("Thermal", &report.timing, &report.runtime.stats, true);
    println!("=======================================================");
}
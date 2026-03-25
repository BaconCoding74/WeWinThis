use crate::common::command_stats::{print_command_report, CommandStats};
use crate::common::cpu_stats::{print_cpu_stats, CpuStats};
use crate::common::metrics::print_task_report;
use crate::common::scheduler_stats::{print_queue_stats, SchedulerStats};
use crate::common::timing::{TaskRuntime, TaskTiming};

pub struct SchedulerReport {
    pub cpu_stats: CpuStats,
    pub command_stats: CommandStats,
    pub scheduler_q_stats: SchedulerStats,

    pub gyro_timing: TaskTiming,
    pub battery_timing: TaskTiming,
    pub compression_timing: TaskTiming,
    pub health_timing: TaskTiming,
    pub antenna_timing: TaskTiming,
    pub cmd_exec_timing: TaskTiming,

    pub gyro_runtime: TaskRuntime,
    pub battery_runtime: TaskRuntime,
    pub compression_runtime: TaskRuntime,
    pub health_runtime: TaskRuntime,
    pub antenna_runtime: TaskRuntime,
    pub cmd_exec_runtime: TaskRuntime,
}

pub fn print_scheduler_report(report: &SchedulerReport) {
    println!("================ Scheduler Report ================");
    print_cpu_stats(&report.cpu_stats);
    print_queue_stats("Scheduler", &report.scheduler_q_stats);

    println!("  gyro runs           : {}", report.gyro_runtime.seq);
    println!("  battery runs        : {}", report.battery_runtime.seq);
    println!("  antenna runs        : {}", report.antenna_runtime.seq);
    println!("  compression runs    : {}", report.compression_runtime.seq);
    println!("  health runs         : {}", report.health_runtime.seq);
    println!("  command exec runs   : {}", report.cmd_exec_runtime.seq);

    print_task_report("Gyro", &report.gyro_timing, &report.gyro_runtime.stats, true);
    println!();
    print_task_report("Battery", &report.battery_timing, &report.battery_runtime.stats, true);
    println!();
    print_task_report("Antenna", &report.antenna_timing, &report.antenna_runtime.stats, true);
    println!();
    print_task_report("Health", &report.health_timing, &report.health_runtime.stats, true);
    println!();
    print_task_report("Command Execution", &report.cmd_exec_timing, &report.cmd_exec_runtime.stats, true);
    println!();
    print_task_report("Compression", &report.compression_timing, &report.compression_runtime.stats, true);
    println!();
    print_command_report("Command", &report.command_stats);
    println!("==================================================");
}
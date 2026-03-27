use std::time::Duration;

use crate::command::{Command, CommandType, ScheduledCommand};

pub fn build_command_schedule(now: std::time::Instant) -> Vec<ScheduledCommand> {
    let scenarios = vec![
        // Initial mode set
        (CommandType::SetDegraded, 0, 0),
        (CommandType::SetNormal, 100, 0),
        // Mode switching tests
        (CommandType::SetDegraded, 200, 0),
        (CommandType::SetNormal, 300, 0),
        (CommandType::SetDegraded, 400, 0),
        (CommandType::SetNormal, 500, 0),
        // Burst mode commands
        (CommandType::SetDegraded, 600, 0),
        (CommandType::SetDegraded, 620, 0),
        (CommandType::SetNormal, 640, 0),
        (CommandType::SetNormal, 660, 0),
        // Mixed with pings
        (CommandType::SetDegraded, 700, 0),
        (CommandType::Ping, 720, 0),
        (CommandType::SetNormal, 740, 0),
        (CommandType::Ping, 760, 0),
        (CommandType::SetDegraded, 780, 0),
        (CommandType::Ping, 800, 0),
        // High frequency mode switching
        (CommandType::SetNormal, 850, 0),
        (CommandType::SetDegraded, 900, 0),
        (CommandType::SetNormal, 950, 0),
        (CommandType::SetDegraded, 1000, 0),
        (CommandType::SetNormal, 1050, 0),
        (CommandType::SetDegraded, 1100, 0),
        // Final mode set
        (CommandType::SetNormal, 1200, 0),
    ];

    let max_delay = scenarios.iter().map(|(_, d, _)| *d).max().unwrap_or(0);
    let _cycle_duration = Duration::from_millis(max_delay + 100);

    scenarios
        .into_iter()
        .map(|(cmd_type, delay_ms, deadline_ms)| {
            let scheduled_time = now + Duration::from_millis(delay_ms);
            ScheduledCommand {
                command: Command { cmd_type, arg: 0 },
                scheduled_time,
                deadline: Duration::from_millis(deadline_ms),
            }
        })
        .collect()
}

pub fn reschedule_cycle(scheduler: &mut crate::command::CommandScheduler, now: std::time::Instant) {
    let scenarios = vec![
        // Initial mode set
        (CommandType::SetDegraded, 0, 0),
        (CommandType::SetNormal, 100, 0),
        // Mode switching tests
        (CommandType::SetDegraded, 200, 0),
        (CommandType::SetNormal, 300, 0),
        (CommandType::SetDegraded, 400, 0),
        (CommandType::SetNormal, 500, 0),
        // Burst mode commands
        (CommandType::SetDegraded, 600, 0),
        (CommandType::SetDegraded, 620, 0),
        (CommandType::SetNormal, 640, 0),
        (CommandType::SetNormal, 660, 0),
        // Mixed with pings
        (CommandType::SetDegraded, 700, 0),
        (CommandType::Ping, 720, 0),
        (CommandType::SetNormal, 740, 0),
        (CommandType::Ping, 760, 0),
        (CommandType::SetDegraded, 780, 0),
        (CommandType::Ping, 800, 0),
        // High frequency mode switching
        (CommandType::SetNormal, 850, 0),
        (CommandType::SetDegraded, 900, 0),
        (CommandType::SetNormal, 950, 0),
        (CommandType::SetDegraded, 1000, 0),
        (CommandType::SetNormal, 1050, 0),
        (CommandType::SetDegraded, 1100, 0),
        // Final mode set
        (CommandType::SetNormal, 1200, 0),
    ];

    for (cmd_type, delay_ms, deadline_ms) in scenarios {
        scheduler.schedule(ScheduledCommand {
            command: Command { cmd_type, arg: 0 },
            scheduled_time: now + Duration::from_millis(delay_ms),
            deadline: Duration::from_millis(deadline_ms),
        });
    }
}

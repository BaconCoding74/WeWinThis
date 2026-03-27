use std::time::Duration;

use crate::command::{Command, CommandType, ScheduledCommand};

pub fn build_command_schedule(now: std::time::Instant) -> Vec<ScheduledCommand> {
    let scenarios = vec![
        (CommandType::Ping, 0, 0),
        (CommandType::Ping, 50, 0),
        (CommandType::Ping, 100, 0),
        (CommandType::Ping, 150, 0),
        (CommandType::Ping, 200, 0),
        (CommandType::SetDegraded, 300, 0),
        (CommandType::Ping, 350, 0),
        (CommandType::Ping, 400, 0),
        (CommandType::SetNormal, 450, 0),
        (CommandType::Ping, 500, 0),
        (CommandType::SetNormal, 550, 0),
        (CommandType::SetNormal, 600, 0),
        (CommandType::Ping, 650, 2),
        (CommandType::Ping, 651, 2),
        (CommandType::Ping, 652, 2),
        (CommandType::Ping, 653, 2),
        (CommandType::Ping, 654, 2),
        (CommandType::Ping, 700, 0),
        (CommandType::Ping, 701, 0),
        (CommandType::Ping, 702, 0),
        (CommandType::Ping, 703, 0),
        (CommandType::Ping, 704, 0),
        (CommandType::Ping, 705, 0),
        (CommandType::Ping, 706, 0),
        (CommandType::Ping, 707, 0),
        (CommandType::SetDegraded, 800, 0),
        (CommandType::Ping, 820, 0),
        (CommandType::Ping, 840, 0),
        (CommandType::SetNormal, 860, 0),
        (CommandType::Ping, 880, 0),
        (CommandType::Ping, 900, 1),
        (CommandType::Ping, 901, 1),
        (CommandType::Ping, 902, 1),
        (CommandType::Ping, 903, 1),
        (CommandType::Ping, 904, 1),
        (CommandType::Ping, 905, 1),
        (CommandType::Ping, 1000, 0),
        (CommandType::Ping, 1100, 0),
        (CommandType::Ping, 1200, 0),
        (CommandType::Ping, 1300, 0),
        (CommandType::Ping, 1400, 0),
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
        (CommandType::Ping, 0, 0),
        (CommandType::Ping, 50, 0),
        (CommandType::Ping, 100, 0),
        (CommandType::Ping, 150, 0),
        (CommandType::Ping, 200, 0),
        (CommandType::SetDegraded, 300, 0),
        (CommandType::Ping, 350, 0),
        (CommandType::Ping, 400, 0),
        (CommandType::SetNormal, 450, 0),
        (CommandType::Ping, 500, 0),
        (CommandType::SetNormal, 550, 0),
        (CommandType::SetNormal, 600, 0),
        (CommandType::Ping, 650, 2),
        (CommandType::Ping, 651, 2),
        (CommandType::Ping, 652, 2),
        (CommandType::Ping, 653, 2),
        (CommandType::Ping, 654, 2),
        (CommandType::Ping, 700, 0),
        (CommandType::Ping, 701, 0),
        (CommandType::Ping, 702, 0),
        (CommandType::Ping, 703, 0),
        (CommandType::Ping, 704, 0),
        (CommandType::Ping, 705, 0),
        (CommandType::Ping, 706, 0),
        (CommandType::Ping, 707, 0),
        (CommandType::SetDegraded, 800, 0),
        (CommandType::Ping, 820, 0),
        (CommandType::Ping, 840, 0),
        (CommandType::SetNormal, 860, 0),
        (CommandType::Ping, 880, 0),
        (CommandType::Ping, 900, 1),
        (CommandType::Ping, 901, 1),
        (CommandType::Ping, 902, 1),
        (CommandType::Ping, 903, 1),
        (CommandType::Ping, 904, 1),
        (CommandType::Ping, 905, 1),
        (CommandType::Ping, 1000, 0),
        (CommandType::Ping, 1100, 0),
        (CommandType::Ping, 1200, 0),
        (CommandType::Ping, 1300, 0),
        (CommandType::Ping, 1400, 0),
    ];

    for (cmd_type, delay_ms, deadline_ms) in scenarios {
        scheduler.schedule(ScheduledCommand {
            command: Command { cmd_type, arg: 0 },
            scheduled_time: now + Duration::from_millis(delay_ms),
            deadline: Duration::from_millis(deadline_ms),
        });
    }
}

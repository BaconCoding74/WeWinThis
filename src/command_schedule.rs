use std::time::Duration;

use crate::command::{Command, CommandType, ScheduledCommand};

pub fn build_command_schedule(now: std::time::Instant) -> Vec<ScheduledCommand> {
    let mut list = Vec::new();

    let scenarios = vec![
        // Scenario 1: latency baseline
        (CommandType::Ping, 0, 0),
        (CommandType::Ping, 50, 0),
        (CommandType::Ping, 100, 0),
        (CommandType::Ping, 150, 0),
        (CommandType::Ping, 200, 0),
        // Scenario 2: mode transitions
        (CommandType::SetDegraded, 300, 0),
        (CommandType::Ping, 350, 0),
        (CommandType::Ping, 400, 0),
        (CommandType::SetNormal, 450, 0),
        (CommandType::Ping, 500, 0),
        // Scenario 3: safety interlock (should reject)
        (CommandType::SetNormal, 550, 0),
        (CommandType::SetNormal, 600, 0),
        // Scenario 4: deadline stress (tight deadlines)
        (CommandType::Ping, 650, 2),
        (CommandType::Ping, 651, 2),
        (CommandType::Ping, 652, 2),
        (CommandType::Ping, 653, 2),
        (CommandType::Ping, 654, 2),
        // Scenario 5: burst load
        (CommandType::Ping, 700, 0),
        (CommandType::Ping, 701, 0),
        (CommandType::Ping, 702, 0),
        (CommandType::Ping, 703, 0),
        (CommandType::Ping, 704, 0),
        (CommandType::Ping, 705, 0),
        (CommandType::Ping, 706, 0),
        (CommandType::Ping, 707, 0),
        // Scenario 6: mixed behavior
        (CommandType::SetDegraded, 800, 0),
        (CommandType::Ping, 820, 0),
        (CommandType::Ping, 840, 0),
        (CommandType::SetNormal, 860, 0),
        (CommandType::Ping, 880, 0),
        // ⚡ Scenario 7: high-frequency jitter test
        (CommandType::Ping, 900, 1),
        (CommandType::Ping, 901, 1),
        (CommandType::Ping, 902, 1),
        (CommandType::Ping, 903, 1),
        (CommandType::Ping, 904, 1),
        (CommandType::Ping, 905, 1),
        // Scenario 8: long-running periodic
        (CommandType::Ping, 1000, 0),
        (CommandType::Ping, 1100, 0),
        (CommandType::Ping, 1200, 0),
        (CommandType::Ping, 1300, 0),
        (CommandType::Ping, 1400, 0),
    ];

    for (cmd_type, delay_ms, deadline_ms) in scenarios {
        list.push(ScheduledCommand {
            command: Command { cmd_type, arg: 0 },
            scheduled_time: now + Duration::from_millis(delay_ms),
            deadline: Duration::from_millis(deadline_ms),
        });
    }

    list
}

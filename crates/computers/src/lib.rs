#[must_use]
pub fn command_shell(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

pub fn read_file_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

use anybot_domain::TaskStatus;
use anybot_foundation::{Clock, Secret, TimestampMs};
use anybot_protocol::RunEvent;

#[derive(Debug)]
pub struct ComputerAction {
    pub event: RunEvent,
    pub at: TimestampMs,
    pub secrets_used: Vec<Secret>,
    pub resulting_status: TaskStatus,
    pub http: Option<reqwest::Client>,
}

impl ComputerAction {
    #[must_use]
    pub fn at_now(event: RunEvent, status: TaskStatus, clock: &dyn Clock) -> Self {
        ComputerAction {
            event,
            at: clock.now_ms(),
            secrets_used: Vec::new(),
            resulting_status: status,
            http: None,
        }
    }
}

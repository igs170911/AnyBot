#[must_use]
pub fn command_shell(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

pub fn read_file_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

use anybot_domain::TaskStatus;
use anybot_foundation::{Secret, now_epoch_seconds};
use anybot_protocol::EventEnvelope;

#[derive(Debug)]
pub struct ComputerAction {
    pub event: EventEnvelope,
    pub at_epoch_seconds: i64,
    pub secrets_used: Vec<Secret>,
    pub resulting_status: TaskStatus,
    pub http: Option<reqwest::Client>,
}

impl ComputerAction {
    #[must_use]
    pub fn at_now(event: EventEnvelope, status: TaskStatus) -> Self {
        ComputerAction {
            event,
            at_epoch_seconds: now_epoch_seconds(),
            secrets_used: Vec::new(),
            resulting_status: status,
            http: None,
        }
    }
}

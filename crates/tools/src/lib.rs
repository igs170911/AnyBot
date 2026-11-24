use anybot_domain::TaskStatus;

use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;

#[derive(Debug, Clone)]
pub struct ToolOutcome {
    pub status: TaskStatus,
    pub event: EventEnvelope,
    pub at_epoch_seconds: i64,
}

impl ToolOutcome {
    #[must_use]
    pub fn new(status: TaskStatus, kind: &str) -> Self {
        ToolOutcome {
            status,
            event: EventEnvelope {
                seq: 0,
                kind: kind.to_owned(),
                at_epoch_seconds: now_epoch_seconds(),
            },
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}

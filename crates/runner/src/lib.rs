use anybot_trust::Evidence;

use anybot_domain::TaskStatus;
use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;

#[derive(Debug, Clone)]
pub struct Turn {
    pub evidence: Vec<Evidence>,
    pub events: Vec<EventEnvelope>,
    pub status: TaskStatus,
    pub started_at_epoch_seconds: i64,
}

impl Turn {
    #[must_use]
    pub fn new(status: TaskStatus) -> Self {
        Turn {
            evidence: Vec::new(),
            events: Vec::new(),
            status,
            started_at_epoch_seconds: now_epoch_seconds(),
        }
    }
}

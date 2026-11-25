use anybot_trust::Evidence;

use anybot_domain::TaskStatus;
use anybot_foundation::{Clock, TimestampMs};
use anybot_protocol::RunEvent;

#[derive(Debug, Clone)]
pub struct Turn {
    pub evidence: Vec<Evidence>,
    pub events: Vec<RunEvent>,
    pub status: TaskStatus,
    pub started_at: TimestampMs,
}

impl Turn {
    #[must_use]
    pub fn new(status: TaskStatus, clock: &dyn Clock) -> Self {
        Turn {
            evidence: Vec::new(),
            events: Vec::new(),
            status,
            started_at: clock.now_ms(),
        }
    }
}

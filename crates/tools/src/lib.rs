use anybot_domain::TaskStatus;

use anybot_foundation::{Clock, TimestampMs};
use anybot_protocol::RunEvent;

#[derive(Debug, Clone)]
pub struct ToolOutcome {
    pub status: TaskStatus,
    pub event: RunEvent,
    pub at: TimestampMs,
}

impl ToolOutcome {
    #[must_use]
    pub fn new(status: TaskStatus, event: RunEvent, clock: &dyn Clock) -> Self {
        ToolOutcome {
            status,
            at: clock.now_ms(),
            event,
        }
    }
}

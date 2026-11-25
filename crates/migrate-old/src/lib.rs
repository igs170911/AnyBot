use anybot_domain::TaskStatus;
use anybot_foundation::{Clock, TimestampMs};
use anybot_protocol::RunEvent;
use anybot_store_client::Pool;

#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub notes: Vec<String>,
    pub event: RunEvent,
    pub at: TimestampMs,
}

impl MigrationReport {
    #[must_use]
    pub fn start(event: RunEvent, clock: &dyn Clock) -> Self {
        MigrationReport {
            notes: Vec::new(),
            event,
            at: clock.now_ms(),
        }
    }
}

pub type TargetPool = Pool;

#[must_use]
pub fn migrated_status_placeholder() -> TaskStatus {
    TaskStatus::Completed
}

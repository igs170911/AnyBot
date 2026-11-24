use anybot_domain::TaskStatus;
use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;
use anybot_store_client::Pool;

#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub notes: Vec<String>,
    pub event: EventEnvelope,
    pub at_epoch_seconds: i64,
}

impl MigrationReport {
    #[must_use]
    pub fn start(event: EventEnvelope) -> Self {
        MigrationReport {
            notes: Vec::new(),
            event,
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}

pub type TargetPool = Pool;

#[must_use]
pub fn migrated_status_placeholder() -> TaskStatus {
    TaskStatus::Done
}

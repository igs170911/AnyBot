use anybot_domain::TaskStatus;
use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;

pub type Pool = sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct ReplicaRecord {
    pub event: EventEnvelope,
    pub status: TaskStatus,
    pub at_epoch_seconds: i64,
}

impl ReplicaRecord {
    #[must_use]
    pub fn new(event: EventEnvelope, status: TaskStatus) -> Self {
        ReplicaRecord {
            event,
            status,
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}

#[must_use]
pub fn db_file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

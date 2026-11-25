use anybot_domain::TaskStatus;
use anybot_foundation as _;
use anybot_protocol::RunEvent;

pub type Pool = sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct ReplicaRecord {
    pub event: RunEvent,
    pub status: TaskStatus,
}

impl ReplicaRecord {
    #[must_use]
    pub fn new(event: RunEvent, status: TaskStatus) -> Self {
        ReplicaRecord { event, status }
    }
}

#[must_use]
pub fn db_file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

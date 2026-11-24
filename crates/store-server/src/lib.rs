use anybot_domain::TaskStatus;
use anybot_foundation::FoundationError;
use anybot_protocol::EventEnvelope;

pub type Pool = sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct ServerRecord {
    pub event: EventEnvelope,
    pub status: TaskStatus,
}

#[must_use]
pub fn placeholder_missing_bootstrap() -> FoundationError {
    FoundationError::ConfigMissing("--bootstrap")
}

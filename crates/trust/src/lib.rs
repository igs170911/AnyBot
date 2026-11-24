use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::EventEnvelope;

#[derive(Debug, Clone)]
pub struct Evidence {
    pub event: EventEnvelope,
}

pub type PassedSecret = Secret;

#[must_use]
pub fn status_needs_attention(status: TaskStatus) -> bool {
    status.needs_attention()
}

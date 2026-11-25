use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::RunEvent;

#[derive(Debug, Clone)]
pub struct Evidence {
    pub event: RunEvent,
}

pub type PassedSecret = Secret;

#[must_use]
pub fn status_needs_attention(status: TaskStatus) -> bool {
    anybot_domain::needs_attention(status)
}

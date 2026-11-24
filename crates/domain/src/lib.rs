#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Waiting,
    Stuck,
    Done,
    Failed,
}

impl TaskStatus {
    #[must_use]
    pub fn needs_attention(self) -> bool {
        matches!(self, TaskStatus::Stuck | TaskStatus::Failed)
    }
}

use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;

#[derive(Debug, Clone)]
pub struct EvidenceItem {
    pub event: EventEnvelope,
    pub at_epoch_seconds: i64,
}

impl EvidenceItem {
    #[must_use]
    pub fn new(event: EventEnvelope) -> Self {
        EvidenceItem {
            event,
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TaskStatus;

    #[test]
    fn stuck_and_failed_need_attention() {
        assert!(TaskStatus::Stuck.needs_attention());
        assert!(TaskStatus::Failed.needs_attention());
        assert!(!TaskStatus::Running.needs_attention());
    }
}

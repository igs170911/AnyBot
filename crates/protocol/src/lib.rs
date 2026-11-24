use anybot_foundation::now_epoch_seconds;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub seq: u64,
    pub kind: String,
    pub at_epoch_seconds: i64,
}

impl EventEnvelope {
    #[must_use]
    pub fn now(seq: u64, kind: &str) -> Self {
        EventEnvelope {
            seq,
            kind: kind.to_owned(),
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}

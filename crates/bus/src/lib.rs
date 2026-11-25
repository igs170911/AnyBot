use anybot_foundation::{Clock, TimestampMs};

#[derive(Debug, Clone)]
pub struct Notification {
    pub kind: String,
    pub at: TimestampMs,
}

impl Notification {
    #[must_use]
    pub fn new(kind: &str, clock: &dyn Clock) -> Self {
        Notification {
            kind: kind.to_owned(),
            at: clock.now_ms(),
        }
    }
}

use anybot_foundation::now_epoch_seconds;

#[derive(Debug, Clone)]
pub struct Notification {
    pub kind: String,
    pub at_epoch_seconds: i64,
}

impl Notification {
    #[must_use]
    pub fn new(kind: &str) -> Self {
        Notification {
            kind: kind.to_owned(),
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}

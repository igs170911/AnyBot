use anybot_foundation::{Clock, TimestampMs};

#[derive(Debug, Clone)]
pub struct BlobRef {
    pub path: String,
    pub at: TimestampMs,
}

impl BlobRef {
    #[must_use]
    pub fn new(path: &str, clock: &dyn Clock) -> Self {
        BlobRef {
            path: path.to_owned(),
            at: clock.now_ms(),
        }
    }
}

pub fn write_local(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

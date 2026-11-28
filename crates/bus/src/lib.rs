use std::sync::Arc;

use tokio::sync::broadcast;

use anybot_foundation::{Clock, TimestampMs};

#[derive(Debug, Clone)]
pub struct Notification {
    pub kind: String,
    pub at: TimestampMs,
    pub seq: Option<i64>,
}

impl Notification {
    #[must_use]
    pub fn new(kind: &str, clock: &dyn Clock) -> Self {
        Notification {
            kind: kind.to_owned(),
            at: clock.now_ms(),
            seq: None,
        }
    }

    #[must_use]
    pub fn with_seq(kind: &str, clock: &dyn Clock, seq: i64) -> Self {
        Notification {
            kind: kind.to_owned(),
            at: clock.now_ms(),
            seq: Some(seq),
        }
    }
}

#[must_use]
pub fn stream_note(stream_id: &str) -> String {
    format!("stream:{stream_id}")
}

#[must_use]
pub fn stream_id_of_note(kind: &str) -> Option<&str> {
    kind.strip_prefix(STREAM_NOTE_PREFIX)
}

pub const STREAM_NOTE_PREFIX: &str = "stream:";
pub const REVOKE_NOTE_PREFIX: &str = "revoked:";

#[must_use]
pub fn revoke_note(account: &str) -> String {
    format!("{REVOKE_NOTE_PREFIX}{account}")
}

#[must_use]
pub fn account_of_revoke(kind: &str) -> Option<&str> {
    kind.strip_prefix(REVOKE_NOTE_PREFIX)
}

pub trait Bus: Send + Sync {
    fn publish(&self, note: Notification);

    fn subscribe(&self) -> broadcast::Receiver<Notification>;
}

pub struct MemoryBus {
    sender: broadcast::Sender<Notification>,
}

impl MemoryBus {
    #[must_use]
    pub fn new() -> Self {
        let (sender, rx) = broadcast::channel(256);
        drop(rx);
        MemoryBus { sender }
    }
}

impl Default for MemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus for MemoryBus {
    fn publish(&self, note: Notification) {
        let _sent: Result<usize, broadcast::error::SendError<Notification>> =
            self.sender.send(note);
    }

    fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.sender.subscribe()
    }
}

pub type SharedBus = Arc<dyn Bus>;

#[must_use]
pub fn shared_memory_bus() -> SharedBus {
    Arc::new(MemoryBus::new())
}

pub mod typing;

pub use typing::{SharedTyping, TypingBus, TypingNote, shared_typing_bus};

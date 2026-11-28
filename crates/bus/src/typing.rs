use std::sync::Arc;

use tokio::sync::broadcast;

use anybot_foundation::TimestampMs;

const TYPING_CAPACITY: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingNote {
    pub stream: String,
    pub task_id: String,
    pub text: String,
    pub thinking: String,
    pub at: TimestampMs,
}

pub struct TypingBus {
    sender: broadcast::Sender<TypingNote>,
}

impl TypingBus {
    #[must_use]
    pub fn new() -> Self {
        let (sender, rx) = broadcast::channel(TYPING_CAPACITY);
        drop(rx);
        TypingBus { sender }
    }

    pub fn publish(&self, note: TypingNote) {
        let _sent: Result<usize, broadcast::error::SendError<TypingNote>> = self.sender.send(note);
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<TypingNote> {
        self.sender.subscribe()
    }
}

impl Default for TypingBus {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedTyping = Arc<TypingBus>;

#[must_use]
pub fn shared_typing_bus() -> SharedTyping {
    Arc::new(TypingBus::new())
}

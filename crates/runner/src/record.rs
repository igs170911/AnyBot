use anybot_foundation::{
    BotId, ContentDigest, DeviceId, EventId, PinBoxFuture, RecordId, TaskId, TimestampMs,
};
use anybot_protocol::{TaskEvent, TaskEventKind};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLine {
    pub record_id: RecordId,
    pub bot: BotId,
    pub task: TaskId,
    pub device: DeviceId,
    pub text: String,
    pub at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTaskEvent {
    pub bot: BotId,
    pub task: TaskId,
    pub turn: Option<u32>,
    pub event_id: EventId,
    pub at_ms: TimestampMs,
    pub kind: TaskEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordEntry {
    UserSaid(UserLine),
    Task(NewTaskEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredBody {
    UserSaid(UserLine),
    Task {
        task: TaskId,
        turn: Option<u32>,
        event: TaskEvent,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    pub stream_seq: u64,
    pub body: StoredBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("record failed: {reason}")]
pub struct RecordError {
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("raw store failed: {reason}")]
pub struct RawStoreError {
    pub reason: String,
}

pub trait Record: Send + Sync {
    fn append(&self, entry: RecordEntry) -> PinBoxFuture<'_, Result<Stored, RecordError>>;

    fn conversation(&self, bot: BotId) -> PinBoxFuture<'_, Result<Vec<Stored>, RecordError>>;

    fn task_events(&self, task: TaskId) -> PinBoxFuture<'_, Result<Vec<TaskEvent>, RecordError>>;
}

pub trait RawStore: Send + Sync {
    fn put(&self, bytes: Vec<u8>) -> PinBoxFuture<'_, Result<ContentDigest, RawStoreError>>;

    fn get(&self, id: ContentDigest) -> PinBoxFuture<'_, Result<Option<Vec<u8>>, RawStoreError>>;
}

#[must_use]
pub fn task_event_of(new: &NewTaskEvent, seq: u64) -> TaskEvent {
    TaskEvent {
        event_id: new.event_id,
        seq,
        at_ms: new.at_ms,
        kind: new.kind.clone(),
    }
}

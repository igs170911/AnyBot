use anybot_foundation::{CallId, TaskId};
use thiserror::Error;

use crate::record::{RawStoreError, RecordError};

#[derive(Debug, Error)]
pub enum TurnError {
    #[error("a turn is already running for this bot")]
    Busy,
    #[error("task {task} was not found")]
    TaskNotFound { task: TaskId },
    #[error("task {task} is closed")]
    TaskClosed { task: TaskId },
    #[error(transparent)]
    Record(#[from] RecordError),
    #[error(transparent)]
    Raw(#[from] RawStoreError),
    #[error(transparent)]
    Compile(#[from] anybot_foundation::FoundationError),
    #[error(transparent)]
    Messages(#[from] anybot_messages::MessagesError),
    #[error(transparent)]
    Encode(#[from] serde_json::Error),
    #[error("message code {code} is not in the catalog")]
    UnknownMessageCode { code: String },
    #[error("tool request {call_id} has no result in the record")]
    UnpairedToolRequest { call_id: CallId },
    #[error("replay gap {0:?}")]
    Replay(anybot_domain::ReplayGap),
    #[error(transparent)]
    Report(#[from] anybot_trust::ReportFail),
    #[error("the parked approval list is busy")]
    ParkedBusy,
    #[error("no parked card matches digest {digest}")]
    ApprovalNotFound { digest: String },
    #[error("the runner is not serving the parked card of digest {digest}")]
    ApprovalForeign { digest: String },
    #[error("no user-initiated turn is running")]
    NoOwnTurn,
}

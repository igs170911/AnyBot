use anybot_foundation::CallId;
use thiserror::Error;

use crate::record::{RawStoreError, RecordError};

#[derive(Debug, Error)]
pub enum TurnError {
    #[error("a turn is already running for this bot")]
    Busy,
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
    #[error("tool request {call_id} has no result in the record")]
    UnpairedToolRequest { call_id: CallId },
}

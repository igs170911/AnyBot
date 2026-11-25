use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use anybot_foundation::{
    BlobId, CallId, CardId, Clock, EventId, EvidenceId, TaskId, TimestampMs, TurnId,
};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Paused,
    AwaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum EventKind {
    TurnStarted {
        #[schema(value_type = String, format = "uuid")]
        turn_id: TurnId,
    },
    TurnText {
        #[schema(value_type = String, format = "uuid")]
        turn_id: TurnId,
        text: String,
    },
    TurnThinking {
        #[schema(value_type = String, format = "uuid")]
        turn_id: TurnId,
        text: String,
    },
    TurnEnded {
        #[schema(value_type = String, format = "uuid")]
        turn_id: TurnId,
    },
    ToolCallStarted {
        #[schema(value_type = String, format = "uuid")]
        turn_id: TurnId,
        #[schema(value_type = String, format = "uuid")]
        call_id: CallId,
        name: String,
        args: serde_json::Value,
    },
    ToolCallResult {
        #[schema(value_type = String, format = "uuid")]
        turn_id: TurnId,
        #[schema(value_type = String, format = "uuid")]
        call_id: CallId,
        ok: bool,
        output: serde_json::Value,
    },
    ApprovalCard {
        #[schema(value_type = String, format = "uuid")]
        turn_id: TurnId,
        #[schema(value_type = String, format = "uuid")]
        card_id: CardId,
        summary: String,
        options: Vec<String>,
    },
    EvidenceAdded {
        #[schema(value_type = String, format = "uuid")]
        turn_id: TurnId,
        #[schema(value_type = String, format = "uuid")]
        evidence_id: EvidenceId,
        summary: String,
        #[schema(value_type = String, format = "uuid")]
        blob_id: BlobId,
    },
    TaskStatusChanged {
        status: TaskStatus,
    },
    RunError {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RunEvent {
    #[schema(value_type = String, format = "uuid")]
    pub event_id: EventId,
    #[schema(value_type = String, format = "uuid")]
    pub task_id: TaskId,
    pub seq: u64,
    #[schema(value_type = u64)]
    pub at_ms: TimestampMs,
    pub protocol_version: u32,
    pub kind: EventKind,
}

impl RunEvent {
    #[must_use]
    pub fn new(clock: &dyn Clock, task_id: TaskId, seq: u64, kind: EventKind) -> Self {
        RunEvent {
            event_id: EventId::generate(clock),
            task_id,
            seq,
            at_ms: clock.now_ms(),
            protocol_version: PROTOCOL_VERSION,
            kind,
        }
    }

    pub fn accept(&self) -> Result<(), String> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(format!("不支援的協定版本 {}", self.protocol_version));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;

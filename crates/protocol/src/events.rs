use crate::versions::accepts_protocol_version;
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
    Running,
    AwaitingUser,
    Stuck,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompletedDetail {
    Delivered,
    UserConfirmed,
    Accepted,
    Unverified,
    Stopped,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum KnownEventKind {
    TurnStarted {},
    TurnText {
        text: String,
    },
    TurnThinking {
        text: String,
    },
    TurnEnded {},
    ToolCallStarted {
        #[schema(value_type = String, format = "uuid")]
        call_id: CallId,
        name: String,
        args: serde_json::Value,
    },
    ToolCallResult {
        #[schema(value_type = String, format = "uuid")]
        call_id: CallId,
        ok: bool,
        output: serde_json::Value,
    },
    ApprovalCard {
        #[schema(value_type = String, format = "uuid")]
        card_id: CardId,
        summary: String,
        options: Vec<String>,
    },
    EvidenceAdded {
        #[schema(value_type = String, format = "uuid")]
        evidence_id: EvidenceId,
        summary: String,
        #[schema(value_type = String, format = "uuid")]
        blob_id: BlobId,
    },
    TaskStatusChanged {
        #[schema(value_type = String, format = "uuid")]
        task_id: TaskId,
        status: TaskStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<CompletedDetail>,
    },
    RunError {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RawEventKind {
    pub kind: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(untagged)]
pub enum EventKind {
    Known(KnownEventKind),
    Raw(RawEventKind),
}

impl EventKind {
    #[must_use]
    pub fn kind_name(&self) -> String {
        match self {
            EventKind::Known(KnownEventKind::TurnStarted { .. }) => "turn_started".to_owned(),
            EventKind::Known(KnownEventKind::TurnText { .. }) => "turn_text".to_owned(),
            EventKind::Known(KnownEventKind::TurnThinking { .. }) => "turn_thinking".to_owned(),
            EventKind::Known(KnownEventKind::TurnEnded { .. }) => "turn_ended".to_owned(),
            EventKind::Known(KnownEventKind::ToolCallStarted { .. }) => {
                "tool_call_started".to_owned()
            }
            EventKind::Known(KnownEventKind::ToolCallResult { .. }) => {
                "tool_call_result".to_owned()
            }
            EventKind::Known(KnownEventKind::ApprovalCard { .. }) => "approval_card".to_owned(),
            EventKind::Known(KnownEventKind::EvidenceAdded { .. }) => "evidence_added".to_owned(),
            EventKind::Known(KnownEventKind::TaskStatusChanged { .. }) => {
                "task_status_changed".to_owned()
            }
            EventKind::Known(KnownEventKind::RunError { .. }) => "run_error".to_owned(),
            EventKind::Raw(raw) => raw.kind.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RunEvent {
    #[schema(value_type = String, format = "uuid")]
    pub event_id: EventId,
    #[schema(value_type = String, format = "uuid")]
    pub turn_id: TurnId,
    #[schema(value_type = u64)]
    pub at_ms: TimestampMs,
    pub protocol_version: u32,
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RecordedEvent {
    pub seq: u64,
    #[serde(flatten)]
    pub event: RunEvent,
}

impl RunEvent {
    #[must_use]
    pub fn new(clock: &dyn Clock, turn_id: TurnId, kind: EventKind) -> Self {
        RunEvent {
            event_id: EventId::generate(clock),
            turn_id,
            at_ms: clock.now_ms(),
            protocol_version: PROTOCOL_VERSION,
            kind,
        }
    }

    #[must_use]
    pub fn recorded(&self, seq: u64) -> RecordedEvent {
        RecordedEvent {
            seq,
            event: self.clone(),
        }
    }

    pub fn accept(&self, clock: &dyn Clock) -> Result<(), String> {
        if accepts_protocol_version(self.protocol_version, clock) {
            return Ok(());
        }
        Err(format!("不支援的協定版本 {}", self.protocol_version))
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;

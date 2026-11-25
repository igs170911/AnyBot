use crate::api::MessageParams;
use crate::versions::accepts_protocol_version;
use serde::{Deserialize, Serialize};

use anybot_foundation::{
    BlobId, CallId, CardId, Clock, EventId, EvidenceId, TaskId, TimestampMs, TurnId,
};

pub const PROTOCOL_VERSION: u32 = 1;
pub const RUN_ERROR_CODE_PROTOCOL_VERSION: &str = "run-error-protocol-version";
pub const RUN_ERROR_CODE_UNSUPPORTED_EVENT: &str = "run-error-unsupported-event";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Running,
    AwaitingUser,
    Stuck,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum CompletedDetail {
    Delivered,
    UserConfirmed,
    Accepted,
    Unverified,
    Stopped,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
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
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
        name: String,
        args: serde_json::Value,
    },
    ToolCallResult {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
        ok: bool,
        output: serde_json::Value,
    },
    ApprovalCard {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        card_id: CardId,
        summary: String,
        options: Vec<String>,
    },
    EvidenceAdded {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        evidence_id: EvidenceId,
        summary: String,
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        blob_id: BlobId,
    },
    TaskStatusChanged {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        task_id: TaskId,
        status: TaskStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<CompletedDetail>,
    },
    RunError {
        #[cfg_attr(feature = "schema", schema(example = "run-error-protocol-version"))]
        code: String,
        #[serde(default, skip_serializing_if = "MessageParams::is_empty")]
        #[cfg_attr(feature = "schema", schema(value_type = MessageParams))]
        params: MessageParams,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RawEventKind {
    pub kind: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RunEvent {
    #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
    pub event_id: EventId,
    #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
    pub turn_id: TurnId,
    #[cfg_attr(feature = "schema", schema(value_type = u64))]
    pub at_ms: TimestampMs,
    pub protocol_version: u32,
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
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
        Err(format!(
            "unsupported protocol version {}",
            self.protocol_version
        ))
    }

    #[must_use]
    pub fn unsupported_version_error(&self) -> crate::api::ProblemJson {
        crate::api::ProblemJson::new(
            400,
            crate::api::PROBLEM_TITLE_BAD_REQUEST.to_owned(),
            format!("{}/bad-request", crate::api::PROBLEM_TYPE_BASE),
            RUN_ERROR_CODE_PROTOCOL_VERSION.to_owned(),
        )
        .with_params(MessageParams::from_pairs([(
            "supported".to_owned(),
            serde_json::Value::from(PROTOCOL_VERSION),
        )]))
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "events_prop_tests.rs"]
mod prop_tests;

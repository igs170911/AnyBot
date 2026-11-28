use serde::{Deserialize, Serialize};

use crate::streams_api::StreamSelector;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", schema(example = "card_answer"))]
pub enum RefereeAction {
    Cancel,
    Stop,
    Wake,
    CardAnswer,
    DoneCriteria,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum WakeReason {
    Resume,
    Verification,
}

impl WakeReason {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            WakeReason::Resume => "resume",
            WakeReason::Verification => "verification",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RefereeRequest {
    pub stream: StreamSelector,
    #[cfg_attr(feature = "schema", schema(example = "018f-task-uuidv7"))]
    pub task_id: String,
    pub action: RefereeAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = "018f-card-uuidv7"))]
    pub card_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = "allow"))]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<WakeReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RefereeReply {
    pub stream: StreamSelector,
    #[cfg_attr(feature = "schema", schema(example = 12_i64))]
    pub seq: i64,
}

use serde::{Deserialize, Serialize};

use crate::streams_api::StreamSelector;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct TypingRequest {
    #[cfg_attr(feature = "schema", schema(example = json!({"bot": {"id": "018f-bot-uuidv7"}})))]
    pub stream: StreamSelector,
    #[cfg_attr(feature = "schema", schema(example = "018f-task-uuidv7"))]
    pub task_id: String,
    #[cfg_attr(feature = "schema", schema(example = "The answer so far"))]
    pub text: String,
    #[cfg_attr(feature = "schema", schema(example = ""))]
    pub thinking: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct TypingHint {
    #[cfg_attr(feature = "schema", schema(example = "bot/018f-bot-uuidv7"))]
    pub stream: String,
    #[cfg_attr(feature = "schema", schema(example = "018f-task-uuidv7"))]
    pub task_id: String,
    #[cfg_attr(feature = "schema", schema(example = "The answer so far"))]
    pub text: String,
    #[cfg_attr(feature = "schema", schema(example = ""))]
    pub thinking: String,
    #[cfg_attr(feature = "schema", schema(example = 1_700_000_000_100_i64))]
    pub at_ms: i64,
}

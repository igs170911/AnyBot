use anybot_foundation::{CallId, DeviceId};
use serde::{Deserialize, Serialize};

use crate::computer_caps::ComputerPlatform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum TaskOpener {
    You,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TurnInitiator {
    You {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        device: DeviceId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ComputerRef {
    pub name: String,
    pub platform: ComputerPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ModelName {
    pub provider: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ThinkingChoice {
    Auto,
    Fast,
    Balanced,
    Deep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Finished,
    NeedsTool,
    Length,
    Refusal,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ToolArgs {
    Json { value: serde_json::Value },
    Broken { raw: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ToolRequest {
    #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
    pub call_id: CallId,
    pub provider_call_id: String,
    pub tool: String,
    pub arguments: ToolArgs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct NativeRef {
    pub provider: String,
    pub model: String,
    pub blob: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct TokenUsage {
    pub input: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub output: u64,
    pub thinking: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum NotRunReason {
    NotWired,
    OverBudget,
    PreviousRefused,
    Stopped,
    ComputerOffline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TurnEndReason {
    Answered,
    ContextBudget { deficit_tokens: Option<u64> },
    ModelFailed { code: String },
}

#[cfg(test)]
#[path = "turn_records_tests.rs"]
mod tests;

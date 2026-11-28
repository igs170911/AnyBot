use anybot_foundation::{CallId, DeviceId};
use serde::{Deserialize, Serialize};

use crate::computer_caps::ComputerPlatform;
use crate::task_detail::{TaskDetail, TaskStatus};
use crate::task_events::{BudgetItem, CheckSource};

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
    Verification,
}

impl TurnInitiator {
    #[must_use]
    pub fn is_unattended(&self) -> bool {
        !matches!(self, TurnInitiator::You { .. })
    }
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
pub struct ModelChoice {
    pub model: ModelName,
    pub reason: String,
    pub fail_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TurnEndReason {
    Answered,
    PendingApproval,
    ContextBudget {
        deficit_tokens: Option<u64>,
    },
    ModelFailed {
        code: String,
    },
    Stopped,
    Cancelled,
    OverBudget {
        item: BudgetItem,
        used: u64,
        limit: u64,
    },
    CardTimedOut,
    ModelUnreachable,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ReportOutcome {
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ReportSummary {
    Text(String),
    Missing { code: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ReportAttention {
    pub tool: String,
    pub args_summary: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ReportStep {
    pub name: String,
    pub args_summary: String,
    pub outcome: ReportOutcome,
    #[cfg_attr(feature = "schema", schema(value_type = Vec<String>, format = "uuid"))]
    pub evidence_ids: Vec<anybot_foundation::EvidenceId>,
    pub verify_source: Option<CheckSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct DeliveryReportRecord {
    pub goal: String,
    pub status: TaskStatus,
    pub detail: Option<TaskDetail>,
    pub summary: ReportSummary,
    pub attention: Vec<ReportAttention>,
    pub steps: Vec<ReportStep>,
}

#[cfg(test)]
#[path = "turn_records_tests.rs"]
mod tests;

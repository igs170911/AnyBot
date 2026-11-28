use serde::{Deserialize, Serialize};

use anybot_foundation::CallId;
use anybot_models::{Calibration, Capability, ToolSpec};
use anybot_protocol::{TaskEvent, ToolEffect};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatRecord {
    pub record_id: String,
    pub text: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepOutcomeKind {
    Completed,
    Failed,
    Unknown,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRecord {
    pub seq: u64,
    pub call_id: CallId,
    pub tool: String,
    pub args_summary: String,
    pub arguments: serde_json::Value,
    pub level: ToolEffect,
    pub unconfirmed: bool,
    pub outcome: StepOutcomeKind,
    pub source_ref: String,
    pub content: String,
    pub exit_code: Option<i64>,
    pub error_at: Option<usize>,
}

impl StepRecord {
    #[must_use]
    pub fn has_output(&self) -> bool {
        matches!(
            self.outcome,
            StepOutcomeKind::Completed | StepOutcomeKind::Failed
        )
    }

    #[must_use]
    pub fn keeps_full(&self) -> bool {
        self.unconfirmed
            || self.outcome == StepOutcomeKind::Unknown
            || self.outcome == StepOutcomeKind::Pending
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    UserSaid,
    ToolEvidence,
    ModelInference,
    OtherBot,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub content: String,
    pub relevance: u32,
    pub source: MemorySource,
    pub candidate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedSnapshot {
    pub version: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPlan {
    pub capability: Capability,
    pub calibration: Calibration,
    pub reserved_output: u64,
}

impl ModelPlan {
    #[must_use]
    pub fn usable(&self) -> u64 {
        self.capability
            .context_tokens
            .saturating_sub(self.reserved_output)
    }
}

#[derive(Debug)]
pub struct BuildInput {
    pub view_limit: usize,
    pub events: Vec<TaskEvent>,
    pub user_turns: Vec<ChatRecord>,
    pub bot_turns: Vec<ChatRecord>,
    pub steps: Vec<StepRecord>,
    pub constraints: Vec<String>,
    pub open_questions: Vec<String>,
    pub tool_specs: Vec<ToolSpec>,
    pub system_prompt: String,
    pub pinned: PinnedSnapshot,
    pub memories: Vec<MemoryRecord>,
    pub prior_chat: Vec<ChatRecord>,
    pub model: ModelPlan,
    pub compact_requested: bool,
}

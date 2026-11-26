use serde::{Deserialize, Serialize};

use anybot_foundation::{EventId, TimestampMs};

use crate::known_task_event_kind::KnownTaskEventKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ToolEffect {
    ReadOnly,
    Repeatable,
    Irreversible,
}

#[must_use]
pub fn effect_level(declared: Option<ToolEffect>) -> ToolEffect {
    declared.unwrap_or(ToolEffect::Irreversible)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum BudgetItem {
    Steps,
    ToolCalls,
    Turns,
    Elapsed,
    Tokens,
    Cost,
    Retries,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum CardAnswer {
    AllowOnce,
    AllowConversation,
    AlwaysAllow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ReconcileOutcome {
    Done,
    NotDone,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum DeviceReportKind {
    Shown,
    ThumbsUp,
    ThumbsDown,
    Opened,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum CheckSource {
    Program,
    Rule,
    Model,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "check", content = "args")]
pub enum Criterion {
    FileExists { path: String },
    FileContains { path: String, needle: String },
    CommandExitZero { command: String },
    PageContains { url: String, needle: String },
    SentCount { count: u32 },
    DeliveredToUser { file: String },
    ModelJudgement { text: String },
}

impl Criterion {
    #[must_use]
    pub fn is_model_only(&self) -> bool {
        matches!(self, Criterion::ModelJudgement { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum Proposer {
    User,
    Bot,
    Routine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "change", content = "args")]
pub enum DoneWhenChange {
    SetItems { items: Vec<Criterion> },
    AddItem { item: Criterion },
    RemoveItem { index: usize },
    ReplaceItem { index: usize, item: Criterion },
    Tighten,
    Loosen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(untagged)]
pub enum TaskEventKind {
    Known(KnownTaskEventKind),
    Raw(RawTaskEventKind),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RawTaskEventKind {
    pub kind: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct TaskEvent {
    #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
    pub event_id: EventId,
    pub seq: u64,
    #[cfg_attr(feature = "schema", schema(value_type = u64))]
    pub at_ms: TimestampMs,
    pub kind: TaskEventKind,
}

impl TaskEvent {
    #[must_use]
    pub fn known(&self) -> Option<&KnownTaskEventKind> {
        match &self.kind {
            TaskEventKind::Known(k) => Some(k),
            TaskEventKind::Raw(_) => None,
        }
    }
}

impl TaskEventKind {
    #[must_use]
    pub fn kind_name(&self) -> String {
        match self {
            TaskEventKind::Known(k) => k.kind_name(),
            TaskEventKind::Raw(raw) => raw.kind.clone(),
        }
    }
}

impl KnownTaskEventKind {
    #[must_use]
    pub fn kind_name(&self) -> String {
        let name = match self {
            KnownTaskEventKind::TaskCreated { .. } => "task_created",
            KnownTaskEventKind::DoneWhenChange { .. } => "done_when_change",
            KnownTaskEventKind::DoneWhenAppliedByUser { .. } => "done_when_applied_by_user",
            KnownTaskEventKind::TaskCancelled { .. } => "task_cancelled",
            KnownTaskEventKind::TaskStopped => "task_stopped",
            KnownTaskEventKind::Wake { .. } => "wake",
            KnownTaskEventKind::TurnStarted { .. } => "turn_started",
            KnownTaskEventKind::TurnEnded { .. } => "turn_ended",
            KnownTaskEventKind::ContextBuilt { .. } => "context_built",
            KnownTaskEventKind::ModelAsked { .. } => "model_asked",
            KnownTaskEventKind::ModelAnswer { .. } => "model_answer",
            KnownTaskEventKind::ModelUsage { .. } => "model_usage",
            KnownTaskEventKind::ToolNotRun { .. } => "tool_not_run",
            KnownTaskEventKind::ModelSwapped { .. } => "model_swapped",
            KnownTaskEventKind::GateResult { .. } => "gate_result",
            KnownTaskEventKind::ToolIntent { .. } => "tool_intent",
            KnownTaskEventKind::ToolStarted { .. } => "tool_started",
            KnownTaskEventKind::ToolCompleted { .. } => "tool_completed",
            KnownTaskEventKind::ToolFailed { .. } => "tool_failed",
            KnownTaskEventKind::OutcomeUnknown { .. } => "outcome_unknown",
            KnownTaskEventKind::Reconciled { .. } => "reconciled",
            KnownTaskEventKind::ActionVerified { .. } => "action_verified",
            KnownTaskEventKind::EvidenceAdded { .. } => "evidence_added",
            KnownTaskEventKind::CriterionPassed { .. } => "criterion_passed",
            KnownTaskEventKind::CriterionFailed { .. } => "criterion_failed",
            KnownTaskEventKind::AcceptanceUnavailable => "acceptance_unavailable",
            KnownTaskEventKind::ClaimChecked { .. } => "claim_checked",
            KnownTaskEventKind::BudgetUsage { .. } => "budget_usage",
            KnownTaskEventKind::BudgetLimit { .. } => "budget_limit",
            KnownTaskEventKind::CardShown { .. } => "card_shown",
            KnownTaskEventKind::CardAnswered { .. } => "card_answered",
            KnownTaskEventKind::DeviceReport { .. } => "device_report",
            KnownTaskEventKind::ModelUnreachable => "model_unreachable",
            KnownTaskEventKind::ComputerOffline => "computer_offline",
            KnownTaskEventKind::GoalPaused { .. } => "goal_paused",
            KnownTaskEventKind::Stalled { .. } => "stalled",
        };
        name.to_owned()
    }
}

#[cfg(test)]
#[path = "task_events_tests.rs"]
mod tests;

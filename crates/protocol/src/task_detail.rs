use serde::{Deserialize, Serialize};

use crate::task_events::BudgetItem;
use anybot_foundation::TimestampMs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Running,
    AwaitingUser,
    Stuck,
    Completed,
    Unaccepted,
    Failed,
    Cancelled,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "detail", content = "params")]
pub enum TaskDetail {
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "current_step", "params": {"step": "reading notes"}})))]
    CurrentStep { step: String },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "queued"})))]
    Queued,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "follow_up", "params": {"number": 1, "next_at": 1_700_000_000_000_i64}})))]
    FollowUp {
        number: u32,
        #[cfg_attr(feature = "schema", schema(value_type = u64))]
        next_at: TimestampMs,
    },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "verifying"})))]
    Verifying,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "planning"})))]
    Planning,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "plan_progress", "params": {"step": 1, "total": 3}})))]
    PlanProgress { step: u32, total: u32 },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "auto_resuming"})))]
    AutoResuming,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "approval_pending", "params": {"action": "delete file"}})))]
    ApprovalPending { action: String },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "question_pending", "params": {"question": "which file?"}})))]
    QuestionPending { question: String },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "plan_approval_pending"})))]
    PlanApprovalPending,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "confirmation_pending", "params": {"step": "send file"}})))]
    ConfirmationPending { step: String },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "timed_out"})))]
    TimedOut,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "resume_pending"})))]
    ResumePending,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "computer_offline"})))]
    ComputerOffline,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "goal_paused", "params": {"reason": "waiting for approval"}})))]
    GoalPaused { reason: String },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "acceptance_decision_pending"})))]
    AcceptanceDecisionPending,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "no_activity", "params": {"minutes": 10}})))]
    NoActivity { minutes: u32 },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "over_budget", "params": {"item": "steps", "used": 10, "limit": 5}})))]
    OverBudget {
        item: BudgetItem,
        used: u64,
        limit: u64,
    },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "goal_stuck", "params": {"reason": "no route"}})))]
    GoalStuck { reason: String },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "plan_stale"})))]
    PlanStale,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "replied"})))]
    Replied,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "accepted"})))]
    Accepted,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "delivered"})))]
    Delivered,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "user_confirmed"})))]
    UserConfirmed,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "acceptance_failed"})))]
    AcceptanceFailed,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "unverified_actions"})))]
    UnverifiedActions,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "step_rejected"})))]
    StepRejected,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "no_visible_result"})))]
    NoVisibleResult,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "error", "params": {"reason": "disk full"}})))]
    Error { reason: String },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "goal_impossible"})))]
    GoalImpossible,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "model_unreachable"})))]
    ModelUnreachable,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "context_budget_insufficient"})))]
    ContextBudgetInsufficient,
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "sent_target", "params": {"sent": 2, "total": 3}})))]
    SentTarget { sent: u32, total: u32 },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "cancelled_reason", "params": {"reason": "user cancelled"}})))]
    CancelledReason { reason: String },
    #[cfg_attr(feature = "schema", schema(example = json!({"detail": "you_stopped"})))]
    YouStopped,
}

impl TaskDetail {
    #[must_use]
    pub fn owner_status(&self) -> TaskStatus {
        match self {
            TaskDetail::CurrentStep { .. }
            | TaskDetail::Queued
            | TaskDetail::FollowUp { .. }
            | TaskDetail::Verifying
            | TaskDetail::Planning
            | TaskDetail::PlanProgress { .. }
            | TaskDetail::AutoResuming => TaskStatus::Running,
            TaskDetail::ApprovalPending { .. }
            | TaskDetail::QuestionPending { .. }
            | TaskDetail::PlanApprovalPending
            | TaskDetail::ConfirmationPending { .. }
            | TaskDetail::TimedOut
            | TaskDetail::ResumePending
            | TaskDetail::ComputerOffline
            | TaskDetail::GoalPaused { .. }
            | TaskDetail::AcceptanceDecisionPending => TaskStatus::AwaitingUser,
            TaskDetail::NoActivity { .. }
            | TaskDetail::OverBudget { .. }
            | TaskDetail::GoalStuck { .. }
            | TaskDetail::PlanStale
            | TaskDetail::ModelUnreachable
            | TaskDetail::ContextBudgetInsufficient => TaskStatus::Stuck,
            TaskDetail::Replied
            | TaskDetail::Accepted
            | TaskDetail::Delivered
            | TaskDetail::UserConfirmed => TaskStatus::Completed,
            TaskDetail::AcceptanceFailed
            | TaskDetail::UnverifiedActions
            | TaskDetail::StepRejected
            | TaskDetail::NoVisibleResult => TaskStatus::Unaccepted,
            TaskDetail::Error { .. }
            | TaskDetail::GoalImpossible
            | TaskDetail::SentTarget { .. } => TaskStatus::Failed,
            TaskDetail::CancelledReason { .. } => TaskStatus::Cancelled,
            TaskDetail::YouStopped => TaskStatus::Stopped,
        }
    }
}

#[must_use]
pub fn detail_matches_status(status: TaskStatus, detail: &TaskDetail) -> bool {
    status == detail.owner_status()
}

#[cfg(test)]
#[path = "task_detail_tests.rs"]
mod tests;

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
    CurrentStep {
        step: String,
    },
    Queued,
    FollowUp {
        number: u32,
        #[cfg_attr(feature = "schema", schema(value_type = u64))]
        next_at: TimestampMs,
    },
    Verifying,
    Planning,
    PlanProgress {
        step: u32,
        total: u32,
    },
    AutoResuming,
    ApprovalPending {
        action: String,
    },
    QuestionPending {
        question: String,
    },
    PlanApprovalPending,
    ConfirmationPending {
        step: String,
    },
    TimedOut,
    ResumePending,
    ComputerOffline,
    GoalPaused {
        reason: String,
    },
    AcceptanceDecisionPending,
    NoActivity {
        minutes: u32,
    },
    OverBudget {
        item: BudgetItem,
        used: u64,
        limit: u64,
    },
    GoalStuck {
        reason: String,
    },
    PlanStale,
    Replied,
    Accepted,
    Delivered,
    UserConfirmed,
    AcceptanceFailed,
    UnverifiedActions,
    StepRejected,
    NoVisibleResult,
    Error {
        reason: String,
    },
    GoalImpossible,
    ModelUnreachable,
    ContextBudgetInsufficient,
    SentTarget {
        sent: u32,
        total: u32,
    },
    CancelledReason {
        reason: String,
    },
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

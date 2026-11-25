use serde::{Deserialize, Serialize};

use anybot_foundation::TimestampMs;
use anybot_protocol::{TaskDetail, TaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyTaskStatus {
    Running,
    AwaitingUser,
    Stuck,
    Completed,
    Failed,
    Delivered,
    Produced,
    EndedUnverified,
    Cancelled,
    Stopped,
    FollowingUp,
    Verifying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyContext {
    pub follow_up_number: u32,
    pub follow_up_next_at: TimestampMs,
}

impl LegacyContext {
    #[must_use]
    pub fn new(follow_up_number: u32, follow_up_next_at: TimestampMs) -> Self {
        LegacyContext {
            follow_up_number,
            follow_up_next_at,
        }
    }
}

#[must_use]
pub fn migrate_status(
    old: LegacyTaskStatus,
    ctx: &LegacyContext,
    reason: &str,
) -> (TaskStatus, TaskDetail) {
    match old {
        LegacyTaskStatus::Running => (
            TaskStatus::Running,
            TaskDetail::CurrentStep {
                step: reason.to_owned(),
            },
        ),
        LegacyTaskStatus::AwaitingUser => (
            TaskStatus::AwaitingUser,
            TaskDetail::ApprovalPending {
                action: reason.to_owned(),
            },
        ),
        LegacyTaskStatus::Stuck => (
            TaskStatus::Stuck,
            TaskDetail::GoalStuck {
                reason: reason.to_owned(),
            },
        ),
        LegacyTaskStatus::Completed => (TaskStatus::Completed, TaskDetail::Accepted),
        LegacyTaskStatus::Failed => (
            TaskStatus::Failed,
            TaskDetail::Error {
                reason: reason.to_owned(),
            },
        ),
        LegacyTaskStatus::Delivered => (TaskStatus::Completed, TaskDetail::Delivered),
        LegacyTaskStatus::Produced => (TaskStatus::Unaccepted, TaskDetail::UnverifiedActions),
        LegacyTaskStatus::EndedUnverified => {
            (TaskStatus::Unaccepted, TaskDetail::UnverifiedActions)
        }
        LegacyTaskStatus::Cancelled => (
            TaskStatus::Cancelled,
            TaskDetail::CancelledReason {
                reason: reason.to_owned(),
            },
        ),
        LegacyTaskStatus::Stopped => (TaskStatus::Stopped, TaskDetail::YouStopped),
        LegacyTaskStatus::FollowingUp => (
            TaskStatus::Running,
            TaskDetail::FollowUp {
                number: ctx.follow_up_number,
                next_at: ctx.follow_up_next_at,
            },
        ),
        LegacyTaskStatus::Verifying => (TaskStatus::Running, TaskDetail::Verifying),
    }
}

#[must_use]
pub fn migrate_all(
    olds: &[LegacyTaskStatus],
    ctx: &LegacyContext,
    reason: &str,
) -> Vec<(TaskStatus, TaskDetail)> {
    olds.iter()
        .map(|old| migrate_status(*old, ctx, reason))
        .collect()
}

#[cfg(test)]
#[path = "old_status_tests.rs"]
mod tests;

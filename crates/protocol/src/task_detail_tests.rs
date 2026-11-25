use super::*;

#[test]
fn test_task_detail_owner_status_covers_every_variant() {
    for detail in all_details() {
        assert!(detail_matches_status(detail.owner_status(), &detail));
    }
}

fn all_details() -> Vec<TaskDetail> {
    let mut details = lifecycle_details();
    details.extend(outcome_details());
    details
}

fn lifecycle_details() -> Vec<TaskDetail> {
    [
        TaskDetail::CurrentStep {
            step: "s".to_owned(),
        },
        TaskDetail::Queued,
        TaskDetail::FollowUp {
            number: 1,
            next_at: TimestampMs::from_millis(2),
        },
        TaskDetail::Verifying,
        TaskDetail::Planning,
        TaskDetail::PlanProgress { step: 1, total: 2 },
        TaskDetail::AutoResuming,
        TaskDetail::ApprovalPending {
            action: "a".to_owned(),
        },
        TaskDetail::QuestionPending {
            question: "q".to_owned(),
        },
        TaskDetail::PlanApprovalPending,
        TaskDetail::ConfirmationPending {
            step: "s".to_owned(),
        },
        TaskDetail::TimedOut,
        TaskDetail::ResumePending,
        TaskDetail::ComputerOffline,
        TaskDetail::GoalPaused {
            reason: "r".to_owned(),
        },
        TaskDetail::AcceptanceDecisionPending,
        TaskDetail::NoActivity { minutes: 15 },
        TaskDetail::OverBudget {
            item: BudgetItem::ToolCalls,
            used: 10,
            limit: 10,
        },
        TaskDetail::GoalStuck {
            reason: "r".to_owned(),
        },
        TaskDetail::PlanStale,
    ]
    .to_vec()
}

fn outcome_details() -> Vec<TaskDetail> {
    [
        TaskDetail::Replied,
        TaskDetail::Accepted,
        TaskDetail::Delivered,
        TaskDetail::UserConfirmed,
        TaskDetail::AcceptanceFailed,
        TaskDetail::UnverifiedActions,
        TaskDetail::StepRejected,
        TaskDetail::NoVisibleResult,
        TaskDetail::Error {
            reason: "r".to_owned(),
        },
        TaskDetail::GoalImpossible,
        TaskDetail::ModelUnreachable,
        TaskDetail::ContextBudgetInsufficient,
        TaskDetail::SentTarget { sent: 1, total: 2 },
        TaskDetail::CancelledReason {
            reason: "r".to_owned(),
        },
        TaskDetail::YouStopped,
    ]
    .to_vec()
}

#[test]
fn test_task_detail_status_pairing_rejects_mismatches() {
    let pairs = [
        (TaskStatus::Running, TaskDetail::Replied),
        (TaskStatus::Completed, TaskDetail::Queued),
        (TaskStatus::Cancelled, TaskDetail::YouStopped),
        (
            TaskStatus::Stopped,
            TaskDetail::CancelledReason {
                reason: "r".to_owned(),
            },
        ),
        (TaskStatus::Unaccepted, TaskDetail::Accepted),
    ];
    for (status, detail) in pairs {
        assert!(!detail_matches_status(status, &detail));
    }
}

#[test]
fn test_task_detail_round_trips_through_json() {
    let detail = TaskDetail::OverBudget {
        item: BudgetItem::Tokens,
        used: 7,
        limit: 5,
    };
    let text = serde_json::to_string(&detail).unwrap();
    let back: TaskDetail = serde_json::from_str(&text).unwrap();
    assert_eq!(back, detail);
    let unit = serde_json::to_string(&TaskDetail::YouStopped).unwrap();
    assert_eq!(unit, "{\"detail\":\"you_stopped\"}");
}

#[test]
fn test_task_status_has_eight_main_states() {
    let all = [
        TaskStatus::Running,
        TaskStatus::AwaitingUser,
        TaskStatus::Stuck,
        TaskStatus::Completed,
        TaskStatus::Unaccepted,
        TaskStatus::Failed,
        TaskStatus::Cancelled,
        TaskStatus::Stopped,
    ];
    assert_eq!(all.len(), 8);
    for status in all {
        let text = serde_json::to_string(&status).unwrap();
        let back: TaskStatus = serde_json::from_str(&text).unwrap();
        assert_eq!(back, status);
    }
}

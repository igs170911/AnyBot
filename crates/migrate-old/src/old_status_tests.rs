use std::collections::BTreeSet;

use anybot_foundation::TimestampMs;
use anybot_protocol::{TaskDetail, TaskStatus, detail_matches_status};

use super::*;

fn ctx() -> LegacyContext {
    LegacyContext::new(2, TimestampMs::from_millis(1_700_000_000_000))
}

fn all_old() -> [LegacyTaskStatus; 12] {
    [
        LegacyTaskStatus::Running,
        LegacyTaskStatus::AwaitingUser,
        LegacyTaskStatus::Stuck,
        LegacyTaskStatus::Completed,
        LegacyTaskStatus::Failed,
        LegacyTaskStatus::Delivered,
        LegacyTaskStatus::Produced,
        LegacyTaskStatus::EndedUnverified,
        LegacyTaskStatus::Cancelled,
        LegacyTaskStatus::Stopped,
        LegacyTaskStatus::FollowingUp,
        LegacyTaskStatus::Verifying,
    ]
}

#[test]
fn t_task_state_6_every_legacy_status_maps_to_a_new_pair() {
    let mapped = migrate_all(&all_old(), &ctx(), "r");
    assert_eq!(mapped.len(), 12);
    for (status, detail) in &mapped {
        assert!(detail_matches_status(*status, detail));
    }
    let statuses: BTreeSet<TaskStatus> = mapped.iter().map(|(status, _)| *status).collect();
    assert_eq!(statuses.len(), 8);
}

#[test]
fn t_task_state_6_specific_legacy_pairs() {
    let cases = [
        (LegacyTaskStatus::FollowingUp, TaskStatus::Running),
        (LegacyTaskStatus::Verifying, TaskStatus::Running),
        (LegacyTaskStatus::Delivered, TaskStatus::Completed),
        (LegacyTaskStatus::Produced, TaskStatus::Unaccepted),
        (LegacyTaskStatus::EndedUnverified, TaskStatus::Unaccepted),
        (LegacyTaskStatus::Cancelled, TaskStatus::Cancelled),
        (LegacyTaskStatus::Stopped, TaskStatus::Stopped),
        (LegacyTaskStatus::Completed, TaskStatus::Completed),
    ];
    for (old, expected) in cases {
        let (status, _) = migrate_status(old, &ctx(), "r");
        assert_eq!(status, expected);
    }
    let (status, detail) = migrate_status(LegacyTaskStatus::FollowingUp, &ctx(), "r");
    assert_eq!(
        detail,
        TaskDetail::FollowUp {
            number: 2,
            next_at: TimestampMs::from_millis(1_700_000_000_000),
        }
    );
    assert_eq!(status, TaskStatus::Running);
}

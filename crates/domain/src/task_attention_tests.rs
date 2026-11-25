use anybot_protocol::TaskStatus;

use super::{badge_count, routine_should_pause};
use crate::needs_attention;

#[test]
fn t_task_state_8_stopped_and_cancelled_are_not_failures() {
    let statuses = [
        TaskStatus::Stopped,
        TaskStatus::Cancelled,
        TaskStatus::Failed,
    ];
    assert!(!routine_should_pause(&statuses, 1));
    assert_eq!(badge_count(&statuses), 1);
    assert!(routine_should_pause(&[TaskStatus::Failed], 1));
    assert!(!needs_attention(TaskStatus::Unaccepted));
}

#[test]
fn test_attention_new_states_keep_the_same_flag() {
    assert!(needs_attention(TaskStatus::AwaitingUser));
    assert!(!needs_attention(TaskStatus::Completed));
    assert!(!needs_attention(TaskStatus::Stopped));
}

use anybot_protocol::{RefereeAction, TaskStatus};

use crate::referee::{is_terminal, provisional_status, referee_record_kind};

#[test]
fn each_referee_action_writes_its_own_record_kind() {
    assert_eq!(referee_record_kind(RefereeAction::Cancel), "referee_cancel");
    assert_eq!(
        referee_record_kind(RefereeAction::CardAnswer),
        "referee_card_answer"
    );
}

#[test]
fn a_pending_cancel_shows_as_cancelled_while_the_task_runs() {
    let shown = provisional_status(TaskStatus::Running, &[RefereeAction::Cancel]);
    assert_eq!(shown, TaskStatus::Cancelled);
}

#[test]
fn a_pending_stop_shows_as_stopped_while_the_task_waits_for_the_user() {
    let shown = provisional_status(TaskStatus::AwaitingUser, &[RefereeAction::Stop]);
    assert_eq!(shown, TaskStatus::Stopped);
}

#[test]
fn a_pending_action_never_changes_a_finished_task() {
    let shown = provisional_status(TaskStatus::Completed, &[RefereeAction::Cancel]);
    assert_eq!(shown, TaskStatus::Completed);
    assert!(is_terminal(TaskStatus::Stopped));
}

#[test]
fn a_pending_wake_leaves_the_status_alone() {
    let shown = provisional_status(TaskStatus::Stuck, &[RefereeAction::Wake]);
    assert_eq!(shown, TaskStatus::Stuck);
}

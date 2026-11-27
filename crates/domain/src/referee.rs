use anybot_protocol::{RefereeAction, TaskStatus};

#[must_use]
pub fn referee_record_kind(action: RefereeAction) -> &'static str {
    match action {
        RefereeAction::Cancel => "referee_cancel",
        RefereeAction::Stop => "referee_stop",
        RefereeAction::Wake => "referee_wake",
        RefereeAction::CardAnswer => "referee_card_answer",
        RefereeAction::DoneCriteria => "referee_done_criteria",
    }
}

#[must_use]
pub fn is_terminal(status: TaskStatus) -> bool {
    !matches!(
        status,
        TaskStatus::Running | TaskStatus::AwaitingUser | TaskStatus::Stuck
    )
}

#[must_use]
pub fn provisional_status(formal: TaskStatus, pending: &[RefereeAction]) -> TaskStatus {
    if is_terminal(formal) {
        return formal;
    }
    if pending.contains(&RefereeAction::Cancel) {
        return TaskStatus::Cancelled;
    }
    if pending.contains(&RefereeAction::Stop) {
        return TaskStatus::Stopped;
    }
    formal
}

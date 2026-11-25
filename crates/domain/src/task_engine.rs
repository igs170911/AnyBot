use anybot_protocol::{KnownTaskEventKind, TaskStatus};

use crate::task_engine_events::{advance_action, advance_criteria};
use crate::task_engine_evidence::{advance_budget, advance_done_when, advance_user_signals};
use crate::task_engine_lifecycle::{advance_cancel, advance_goal, advance_lifecycle, advance_turn};
use crate::task_view::TaskView;

pub fn advance(view: &mut TaskView, event: &KnownTaskEventKind) {
    if view.status == TaskStatus::Completed && locks_completion(event) {
        return;
    }
    if advance_goal(view, event) {
        return;
    }
    if advance_cancel(view, event) {
        return;
    }
    if advance_lifecycle(view, event) {
        return;
    }
    if advance_turn(view, event) {
        return;
    }
    if advance_action(view, event) {
        return;
    }
    if advance_criteria(view, event) {
        return;
    }
    if advance_budget(view, event) {
        return;
    }
    if advance_user_signals(view, event) {
        return;
    }
    advance_done_when(view, event);
}

#[must_use]
pub fn locks_completion(event: &KnownTaskEventKind) -> bool {
    matches!(
        event,
        KnownTaskEventKind::OutcomeUnknown { .. }
            | KnownTaskEventKind::CriterionFailed { .. }
            | KnownTaskEventKind::AcceptanceUnavailable
            | KnownTaskEventKind::ClaimChecked { .. }
    )
}

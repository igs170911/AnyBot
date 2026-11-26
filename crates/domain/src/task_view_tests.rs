use anybot_protocol::{BudgetItem, KnownTaskEventKind, TaskDetail, TaskStatus};

use super::initial_view;
use crate::task_budget::{apply_limit, note_usage};

#[test]
fn test_view_initial_state_is_running_and_queued() {
    let view = initial_view();
    assert_eq!(view.status, TaskStatus::Running);
    assert_eq!(view.detail, Some(TaskDetail::Queued));
    assert!(view.completion_ready());
}

#[test]
fn test_budget_limit_absence_means_no_cap() {
    let mut view = initial_view();
    assert!(!view.budget.limits.contains_key(&BudgetItem::Tokens));
    note_usage(&mut view, BudgetItem::Tokens, 10_000);
    assert!(view.budget.over.is_none());
    assert_eq!(view.status, TaskStatus::Running);
}

#[test]
fn test_budget_usage_over_limit_reports_pair() {
    let mut view = initial_view();
    apply_limit(&mut view, BudgetItem::Tokens, 5);
    note_usage(&mut view, BudgetItem::Tokens, 5);
    assert!(view.budget.over.is_none());
    note_usage(&mut view, BudgetItem::Tokens, 1);
    assert_eq!(
        view.budget.over,
        Some(super::BudgetOver {
            item: BudgetItem::Tokens,
            used: 6,
            limit: 5,
        })
    );
    assert_eq!(view.status, TaskStatus::Stuck);
}

#[test]
fn test_budget_attempt_at_cap_stops_new_action() {
    let mut view = initial_view();
    apply_limit(&mut view, BudgetItem::ToolCalls, 2);
    note_usage(&mut view, BudgetItem::ToolCalls, 2);
    assert_eq!(view.status, TaskStatus::Running);
    crate::task_engine::advance(
        &mut view,
        &KnownTaskEventKind::ToolIntent {
            call_id: anybot_foundation::CallId::generate(&anybot_foundation::SystemClock),
            name: "n".to_owned(),
            args_summary: "a".to_owned(),
            level: anybot_protocol::ToolEffect::ReadOnly,
        },
    );
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(view.budget.used(BudgetItem::ToolCalls), 2);
}

#[test]
fn test_view_unverified_action_detection() {
    let mut view = initial_view();
    view.pending_actions.insert("call-1".to_owned());
    assert!(view.unverified_actions());
    view.verified_actions.insert("call-1".to_owned());
    assert!(!view.unverified_actions());
}

#[test]
fn test_completed_detail_ranks_confirm_above_delivery() {
    let mut view = initial_view();
    view.evidence.turn_reply = true;
    view.evidence.delivered_reported = true;
    assert_eq!(view.completed_detail(), TaskDetail::Delivered);
    view.report.user_confirmed = true;
    assert_eq!(view.completed_detail(), TaskDetail::UserConfirmed);
}

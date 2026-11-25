use anybot_protocol::{BudgetItem, TaskDetail, TaskStatus};

use super::{BudgetState, initial_view};

#[test]
fn test_view_initial_state_is_running_and_queued() {
    let view = initial_view();
    assert_eq!(view.status, TaskStatus::Running);
    assert_eq!(view.detail, Some(TaskDetail::Queued));
    assert!(view.completion_ready());
}

#[test]
fn test_budget_limit_absence_means_no_cap() {
    let budget = BudgetState::new();
    assert!(budget.over_item(BudgetItem::Tokens).is_none());
    assert!(budget.first_over().is_none());
}

#[test]
fn test_budget_usage_over_limit_reports_pair() {
    let mut budget = BudgetState::new();
    budget.limits.insert(BudgetItem::Tokens, 5);
    budget.add_usage(BudgetItem::Tokens, 6);
    assert_eq!(budget.over_item(BudgetItem::Tokens), Some((6, 5)));
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

use anybot_foundation::{CallId, SystemClock};
use anybot_protocol::{BudgetItem, KnownTaskEventKind, TaskDetail, TaskStatus, ToolEffect};

use crate::task_engine::advance;
use crate::task_view::{TaskView, initial_view};

fn drive(kinds: &[KnownTaskEventKind]) -> TaskView {
    let mut view = initial_view();
    for kind in kinds {
        advance(&mut view, kind);
    }
    view
}

fn call() -> CallId {
    CallId::generate(&SystemClock)
}

fn created() -> KnownTaskEventKind {
    KnownTaskEventKind::TaskCreated {
        goal: "g".to_owned(),
    }
}

fn starts(n: usize) -> Vec<KnownTaskEventKind> {
    (0..n)
        .map(|_| KnownTaskEventKind::ToolStarted { call_id: call() })
        .collect()
}

#[test]
fn t_task_budget_1() {
    let view = drive(&[
        created(),
        KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::Tokens,
            limit: 1000,
        },
        KnownTaskEventKind::BudgetUsage {
            item: BudgetItem::Tokens,
            delta: 1001,
        },
    ]);
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(
        view.detail,
        Some(TaskDetail::OverBudget {
            item: BudgetItem::Tokens,
            used: 1001,
            limit: 1000,
        })
    );
}

#[test]
fn t_task_budget_2() {
    let mut kinds = vec![
        created(),
        KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::ToolCalls,
            limit: 10,
        },
    ];
    for _ in 0..10 {
        kinds.push(KnownTaskEventKind::ToolIntent {
            call_id: call(),
            name: "n".to_owned(),
            args_summary: "a".to_owned(),
            level: ToolEffect::ReadOnly,
        });
        kinds.push(KnownTaskEventKind::ToolStarted { call_id: call() });
    }
    kinds.push(KnownTaskEventKind::ToolIntent {
        call_id: call(),
        name: "n".to_owned(),
        args_summary: "a".to_owned(),
        level: ToolEffect::ReadOnly,
    });
    let view = drive(&kinds);
    assert_eq!(view.budget.used(BudgetItem::ToolCalls), 10);
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(
        view.detail,
        Some(TaskDetail::OverBudget {
            item: BudgetItem::ToolCalls,
            used: 10,
            limit: 10,
        })
    );
}

#[test]
fn t_task_budget_3() {
    let view = drive(&[
        created(),
        KnownTaskEventKind::BudgetUsage {
            item: BudgetItem::Turns,
            delta: 6,
        },
        KnownTaskEventKind::ModelSwapped {
            from: "a".to_owned(),
            to: "b".to_owned(),
            reason: "r".to_owned(),
        },
        KnownTaskEventKind::BudgetUsage {
            item: BudgetItem::Turns,
            delta: 2,
        },
    ]);
    assert_eq!(view.budget.used(BudgetItem::Turns), 8);
}

#[test]
fn t_task_budget_4() {
    let view = drive(&[
        created(),
        KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::ToolCalls,
            limit: 10,
        },
    ]);
    let mut view = view;
    for kind in starts(10) {
        advance(&mut view, &kind);
    }
    assert_eq!(view.status, TaskStatus::Stuck);
    advance(
        &mut view,
        &KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::ToolCalls,
            limit: 12,
        },
    );
    assert_eq!(view.status, TaskStatus::Running);
    assert_eq!(view.budget.used(BudgetItem::ToolCalls), 10);
}

#[test]
fn t_task_budget_5() {
    let mut kinds = vec![
        created(),
        KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::Retries,
            limit: 3,
        },
    ];
    for _ in 0..4 {
        kinds.push(KnownTaskEventKind::ToolIntent {
            call_id: call(),
            name: "send".to_owned(),
            args_summary: "same".to_owned(),
            level: ToolEffect::Irreversible,
        });
        kinds.push(KnownTaskEventKind::ToolFailed {
            call_id: call(),
            name: "send".to_owned(),
            args_summary: "same".to_owned(),
            error: "e".to_owned(),
        });
    }
    let view = drive(&kinds);
    assert_eq!(view.budget.step_fails.get("send:same").copied(), Some(4));
    assert!(view.gate.step_failed);
    assert_eq!(view.status, TaskStatus::Running);
}

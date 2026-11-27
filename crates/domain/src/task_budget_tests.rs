use anybot_foundation::SystemClock;
use anybot_protocol::{BudgetItem, KnownTaskEventKind, TaskDetail, TaskStatus, ToolEffect};

use crate::task_budget::fixtures::{asks, call, created, drive, intent, pair};

use crate::task_engine::advance;
use crate::task_view::UNLIMITED;

#[must_use]
pub fn turn_started_kind() -> KnownTaskEventKind {
    KnownTaskEventKind::TurnStarted {
        turn: 1,
        initiator: anybot_protocol::TurnInitiator::You {
            device: anybot_foundation::DeviceId::generate(&SystemClock),
        },
        computer: anybot_protocol::ComputerRef {
            name: "c".to_owned(),
            platform: anybot_protocol::ComputerPlatform::BotMachine,
        },
        model: anybot_protocol::ModelName {
            provider: "OpenAi".to_owned(),
            base_url: "http://127.0.0.1:9/v1".to_owned(),
            model: "m".to_owned(),
        },
        thinking: anybot_protocol::ThinkingChoice::Auto,
    }
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
    kinds.extend(pair(10));
    kinds.push(intent());
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
            item: BudgetItem::ToolCalls,
            delta: 6,
        },
        KnownTaskEventKind::ModelSwapped {
            from: "a".to_owned(),
            to: "b".to_owned(),
            reason: "r".to_owned(),
        },
        KnownTaskEventKind::BudgetUsage {
            item: BudgetItem::ToolCalls,
            delta: 2,
        },
    ]);
    assert_eq!(view.budget.used(BudgetItem::ToolCalls), 8);
    assert_ne!(view.status, TaskStatus::Stuck);
}

#[test]
fn t_task_budget_4() {
    let mut kinds = vec![
        created(),
        KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::ToolCalls,
            limit: 10,
        },
    ];
    kinds.extend(pair(10));
    kinds.push(intent());
    let mut view = drive(&kinds);
    assert_eq!(view.status, TaskStatus::Stuck);
    advance(
        &mut view,
        &KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::ToolCalls,
            limit: 20,
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

#[test]
fn t_task_budget_6() {
    let view = drive(&[created()]);
    assert_eq!(
        view.budget.limits.get(&BudgetItem::Steps).copied(),
        Some(300)
    );
    assert_eq!(
        view.budget.limits.get(&BudgetItem::ToolCalls).copied(),
        Some(600)
    );
    assert_eq!(
        view.budget.limits.get(&BudgetItem::Elapsed).copied(),
        Some(4 * 3_600)
    );
    assert!(!view.budget.limits.contains_key(&BudgetItem::Tokens));
    assert!(!view.budget.limits.contains_key(&BudgetItem::Cost));

    let mut view = view;
    advance(
        &mut view,
        &KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::Steps,
            limit: UNLIMITED,
        },
    );
    advance(&mut view, &turn_started_kind());
    for kind in asks(50_000) {
        advance(&mut view, &kind);
    }
    assert_eq!(view.budget.used(BudgetItem::Steps), 50_000);
    assert_ne!(view.status, TaskStatus::Stuck);
    advance(
        &mut view,
        &KnownTaskEventKind::BudgetUsage {
            item: BudgetItem::Steps,
            delta: 1,
        },
    );
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(
        view.detail,
        Some(TaskDetail::OverBudget {
            item: BudgetItem::Steps,
            used: 50_000,
            limit: 50_000,
        })
    );
}

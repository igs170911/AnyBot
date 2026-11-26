use anybot_foundation::SystemClock;
use anybot_protocol::{BudgetItem, KnownTaskEventKind, TaskDetail, TaskStatus, ToolEffect};

use crate::task_budget::fixtures::{asks, call, created, drive, intent, pair, turns_ended};

use crate::task_engine::advance;
use crate::task_view::{TaskView, UNLIMITED, initial_view, mark_goal_task};

fn turn_started_kind() -> KnownTaskEventKind {
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

fn drive_kinds(view: &mut TaskView, kinds: &[KnownTaskEventKind]) {
    for kind in kinds {
        advance(view, kind);
    }
}

fn goal_budget_rounds() -> TaskView {
    let mut view = initial_view();
    mark_goal_task(&mut view);
    let mut kinds = vec![created(), turn_started_kind()];
    kinds.extend(asks(200));
    kinds.push(turns_ended());
    kinds.push(turn_started_kind());
    kinds.extend(asks(200));
    drive_kinds(&mut view, &kinds);
    view
}

fn plain_budget_rounds() -> TaskView {
    let mut view = initial_view();
    let mut kinds = vec![created(), turn_started_kind()];
    kinds.extend(asks(200));
    kinds.push(turns_ended());
    kinds.push(turn_started_kind());
    kinds.extend(asks(100));
    drive_kinds(&mut view, &kinds);
    view
}

fn over_detail(item: BudgetItem, used: u64, limit: u64) -> TaskDetail {
    TaskDetail::OverBudget { item, used, limit }
}

#[test]
fn t_task_budget_9() {
    let mut view = goal_budget_rounds();
    assert_eq!(view.status, TaskStatus::Running);
    assert_eq!(view.budget.used(BudgetItem::Steps), 400);
    drive_kinds(&mut view, &asks(100));
    assert_eq!(view.status, TaskStatus::Running);
    assert_eq!(view.budget.used(BudgetItem::Steps), 500);
    advance(&mut view, &ask_once());
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(view.detail, Some(over_detail(BudgetItem::Steps, 300, 300)));
    assert_eq!(view.budget.used(BudgetItem::Steps), 500);

    let mut plain = plain_budget_rounds();
    assert_eq!(plain.status, TaskStatus::Running);
    advance(&mut plain, &ask_once());
    assert_eq!(plain.status, TaskStatus::Stuck);
    assert_eq!(plain.detail, Some(over_detail(BudgetItem::Steps, 300, 300)));
}

fn ask_once() -> KnownTaskEventKind {
    KnownTaskEventKind::BudgetUsage {
        item: BudgetItem::Steps,
        delta: 1,
    }
}

fn time_events() -> Vec<anybot_protocol::TaskEvent> {
    use anybot_foundation::{EventId, TimestampMs};
    use anybot_protocol::{TaskEvent, TaskEventKind};
    let mut out = vec![];
    let mut push = |seq: u64, at: u64, kind: KnownTaskEventKind| {
        out.push(TaskEvent {
            event_id: EventId::generate(&SystemClock),
            seq,
            at_ms: TimestampMs::from_millis(at),
            kind: TaskEventKind::Known(kind),
        });
    };
    push(1, 0, created());
    push(2, 1_000, turn_started_kind());
    push(
        3,
        2_000,
        KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::Elapsed,
            limit: 3_600,
        },
    );
    push(
        4,
        3_000,
        KnownTaskEventKind::CardShown {
            action: "a".to_owned(),
        },
    );
    push(
        5,
        3_600_000,
        KnownTaskEventKind::CardAnswered {
            answer: anybot_protocol::CardAnswer::AllowOnce,
        },
    );
    push(6, 3_604_000, turns_ended());
    push(7, 3_605_000, turn_started_kind());
    push(
        8,
        3_606_000,
        KnownTaskEventKind::CardShown {
            action: "b".to_owned(),
        },
    );
    push(9, 4_606_000, turns_ended());
    out
}

#[test]
fn time_counts_only_active_turn_spans() {
    use std::collections::BTreeSet;
    let events = time_events();
    let mut view = initial_view();
    let mut seen = BTreeSet::new();
    crate::task_replay::fold(&mut view, &events, &mut seen);
    assert_eq!(view.budget.used(BudgetItem::Elapsed), 7);
    assert_ne!(view.status, TaskStatus::Stuck);
}

use anybot_foundation::SystemClock;
use anybot_protocol::{BudgetItem, KnownTaskEventKind, TaskDetail, TaskStatus};

use crate::task_budget::fixtures::{asks, created, turns_ended};

use crate::task_engine::advance;
use crate::task_view::{TaskView, initial_view, mark_goal_task};

use super::tests::*;

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

fn card_shown(action: &str, card: &str) -> KnownTaskEventKind {
    KnownTaskEventKind::CardShown {
        action: action.to_owned(),
        card: card.to_owned(),
        kind: anybot_protocol::CardKind::Action,
        resume: None,
        buttons: vec![
            anybot_protocol::CardButton::AllowOnce,
            anybot_protocol::CardButton::Deny,
        ],
        reason: anybot_protocol::ReasonPayload {
            code: "card-run-command".to_owned(),
            params: Vec::new(),
        },
        always_label: None,
        notes: Vec::new(),
        classes: Vec::new(),
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
    push(4, 3_000, card_shown("a", "card-1"));
    push(
        5,
        3_600_000,
        KnownTaskEventKind::CardAnswered {
            answer: anybot_protocol::CardAnswer::AllowOnce,
            origin: anybot_protocol::AnswerOrigin::Hand,
        },
    );
    push(6, 3_604_000, turns_ended());
    push(7, 3_605_000, turn_started_kind());
    push(8, 3_606_000, card_shown("b", "card-1"));
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

fn ts(millis: u64) -> anybot_foundation::TimestampMs {
    anybot_foundation::TimestampMs::from_millis(millis)
}

#[test]
fn t_task_budget_8() {
    let mut view = initial_view();
    let kinds = vec![
        created(),
        KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::Elapsed,
            limit: 480,
        },
    ];
    for kind in &kinds {
        advance(&mut view, kind);
    }
    let turn_start = turn_started_kind();
    advance(&mut view, &turn_start);
    let card = KnownTaskEventKind::CardShown {
        action: "shell".to_owned(),
        card: "d".to_owned(),
        kind: anybot_protocol::CardKind::Action,
        resume: None,
        buttons: vec![
            anybot_protocol::CardButton::AllowOnce,
            anybot_protocol::CardButton::Deny,
        ],
        reason: anybot_protocol::ReasonPayload {
            code: "card-run-command".to_owned(),
            params: Vec::new(),
        },
        always_label: None,
        notes: Vec::new(),
        classes: Vec::new(),
    };
    advance(&mut view, &card);
    let answer = KnownTaskEventKind::CardAnswered {
        answer: anybot_protocol::CardAnswer::AllowOnce,
        origin: anybot_protocol::AnswerOrigin::Hand,
    };
    advance(&mut view, &answer);
    crate::task_budget::note_time(&mut view, ts(1_000), &turn_start);
    crate::task_budget::note_time(&mut view, ts(2_000), &card);
    crate::task_budget::note_time(&mut view, ts(1_802_000), &answer);
    let end = KnownTaskEventKind::TurnEnded {
        reason: anybot_protocol::TurnEndReason::Answered,
        reply_delivered: true,
    };
    advance(&mut view, &end);
    crate::task_budget::note_time(&mut view, ts(1_803_000), &end);
    assert_eq!(view.budget.used(BudgetItem::Elapsed), 2);
    assert_ne!(view.status, TaskStatus::Stuck);
    crate::task_budget::refresh_budget_status(&mut view);
    assert_ne!(view.status, TaskStatus::Stuck);
    crate::task_budget::note_usage(&mut view, BudgetItem::Elapsed, 1_800);
    assert_eq!(view.budget.used(BudgetItem::Elapsed), 1_802);
}

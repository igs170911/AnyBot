use anybot_protocol::{BudgetItem, KnownTaskEventKind, TaskDetail, TaskStatus};

use crate::task_budget::fixtures::{created, drive, ev_at, fold_events, intent};
use crate::task_engine::advance;
use crate::task_view::{TaskView, initial_view};

fn turn_started_kind() -> KnownTaskEventKind {
    KnownTaskEventKind::TurnStarted {
        turn: 1,
        initiator: anybot_protocol::TurnInitiator::You {
            device: anybot_foundation::DeviceId::generate(&anybot_foundation::SystemClock),
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

fn ask() -> KnownTaskEventKind {
    KnownTaskEventKind::BudgetUsage {
        item: BudgetItem::Steps,
        delta: 1,
    }
}

fn timed(kinds: &[(u64, KnownTaskEventKind)]) -> Vec<anybot_protocol::TaskEvent> {
    (1usize..=kinds.len())
        .zip(kinds)
        .map(|(seq, (at, kind))| ev_at(u64::try_from(seq).unwrap_or(0), *at, kind.clone()))
        .collect()
}

fn over_detail(item: BudgetItem, used: u64, limit: u64) -> TaskDetail {
    TaskDetail::OverBudget { item, used, limit }
}

fn limit(item: BudgetItem, value: u64) -> KnownTaskEventKind {
    KnownTaskEventKind::BudgetLimit { item, limit: value }
}

fn usage(item: BudgetItem, delta: u64) -> KnownTaskEventKind {
    KnownTaskEventKind::BudgetUsage { item, delta }
}

fn drive_view(kinds: &[KnownTaskEventKind]) -> TaskView {
    let mut view = initial_view();
    for kind in kinds {
        advance(&mut view, kind);
    }
    view
}

#[test]
fn elapsed_limit_blocks_mid_turn_ask() {
    let events = timed(&[
        (0, created()),
        (1_000, turn_started_kind()),
        (1_000 + 14_401_000, ask()),
    ]);
    let view = fold_events(&events);
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(
        view.detail,
        Some(over_detail(BudgetItem::Elapsed, 14_401, 14_400))
    );
}

#[test]
fn card_wait_not_counted_mid_turn() {
    let events = timed(&[
        (0, created()),
        (1_000, turn_started_kind()),
        (
            1_000,
            KnownTaskEventKind::CardShown {
                action: "a".to_owned(),
                card: "card-1".to_owned(),
                kind: anybot_protocol::CardKind::Action,
                resume: None,
            },
        ),
        (
            1_000 + 10_800_000,
            KnownTaskEventKind::CardAnswered {
                answer: anybot_protocol::CardAnswer::AllowOnce,
            },
        ),
        (1_000 + 14_401_000, ask()),
    ]);
    let view = fold_events(&events);
    assert_ne!(view.status, TaskStatus::Stuck);
    assert_eq!(view.budget.used(BudgetItem::Elapsed), 0);
}

#[test]
fn card_open_mid_turn_not_counted() {
    let events = timed(&[
        (0, created()),
        (1_000, turn_started_kind()),
        (
            1_000,
            KnownTaskEventKind::CardShown {
                action: "a".to_owned(),
                card: "card-1".to_owned(),
                kind: anybot_protocol::CardKind::Action,
                resume: None,
            },
        ),
        (1_000 + 14_401_000, ask()),
    ]);
    let view = fold_events(&events);
    assert_ne!(view.status, TaskStatus::Stuck);
}

#[test]
fn token_ask_at_cap_stops() {
    let mut view = drive_view(&[created(), limit(BudgetItem::Tokens, 1_000)]);
    advance(&mut view, &usage(BudgetItem::Tokens, 1_000));
    assert_eq!(view.status, TaskStatus::Running);
    advance(&mut view, &ask());
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(
        view.detail,
        Some(over_detail(BudgetItem::Tokens, 1_000, 1_000))
    );
}

#[test]
fn cost_ask_at_cap_stops() {
    let mut view = drive_view(&[created(), limit(BudgetItem::Cost, 50)]);
    advance(&mut view, &usage(BudgetItem::Cost, 50));
    assert_eq!(view.status, TaskStatus::Running);
    advance(&mut view, &ask());
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(view.detail, Some(over_detail(BudgetItem::Cost, 50, 50)));
}

#[test]
fn token_action_at_cap_stops() {
    let view = drive(&[
        created(),
        limit(BudgetItem::Tokens, 1_000),
        usage(BudgetItem::Tokens, 1_000),
        intent(),
    ]);
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(
        view.detail,
        Some(over_detail(BudgetItem::Tokens, 1_000, 1_000))
    );
}

#[test]
fn cost_action_at_cap_stops() {
    let view = drive(&[
        created(),
        limit(BudgetItem::Cost, 50),
        usage(BudgetItem::Cost, 50),
        intent(),
    ]);
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(view.detail, Some(over_detail(BudgetItem::Cost, 50, 50)));
}

use anybot_foundation::{EventId, SystemClock, TimestampMs};
use anybot_protocol::{
    KnownTaskEventKind, TaskDetail, TaskEvent, TaskEventKind, TaskStatus, TurnEndReason,
};

use super::replay;

fn created() -> KnownTaskEventKind {
    KnownTaskEventKind::TaskCreated {
        goal: "g".to_owned(),
        opened_by: anybot_protocol::TaskOpener::You,
        bot: anybot_foundation::BotId::generate(&SystemClock),
        parent: None,
    }
}

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

fn ev(seq: u64, kind: KnownTaskEventKind) -> TaskEvent {
    TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq,
        at_ms: TimestampMs::from_millis(seq),
        kind: TaskEventKind::Known(kind),
    }
}

fn turn_ended(reason: TurnEndReason, delivered: bool) -> KnownTaskEventKind {
    KnownTaskEventKind::TurnEnded {
        reason,
        reply_delivered: delivered,
    }
}

fn failed(code: &str) -> TurnEndReason {
    TurnEndReason::ModelFailed {
        code: code.to_owned(),
    }
}

#[test]
fn turn_end_context_budget_sets_stuck() {
    let events = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(
            3,
            turn_ended(
                TurnEndReason::ContextBudget {
                    deficit_tokens: Some(7),
                },
                false,
            ),
        ),
    ];
    let view = replay(&events).unwrap();
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(view.detail, Some(TaskDetail::ContextBudgetInsufficient));
}

#[test]
fn turn_end_model_failed_sets_failed() {
    let events = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(3, turn_ended(failed("model-x"), false)),
    ];
    let view = replay(&events).unwrap();
    assert_eq!(view.status, TaskStatus::Failed);
    assert_eq!(
        view.detail,
        Some(TaskDetail::Error {
            reason: "model-x".to_owned(),
        })
    );
}

#[test]
fn turn_end_after_cancel_changes_nothing() {
    let base = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(
            3,
            KnownTaskEventKind::TaskCancelled {
                reason: "r".to_owned(),
            },
        ),
    ];
    for reason in [
        TurnEndReason::Answered,
        failed("model-y"),
        TurnEndReason::ContextBudget {
            deficit_tokens: None,
        },
    ] {
        let mut events = base.clone();
        events.push(ev(4, turn_ended(reason, true)));
        let after = replay(&events).unwrap();
        assert_eq!(after.status, TaskStatus::Cancelled);
        assert_eq!(
            after.detail,
            Some(TaskDetail::CancelledReason {
                reason: "r".to_owned(),
            })
        );
    }
}

use anybot_foundation::{CallId, EventId, SystemClock, TimestampMs};
use anybot_protocol::{
    BudgetItem, CardKind, CheckSource, Criterion, DoneWhenChange, KnownTaskEventKind, Proposer,
    TaskDetail, TaskEvent, TaskEventKind, TaskStatus, ToolEffect, TurnEndReason,
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

fn turn_ended(reason: TurnEndReason) -> KnownTaskEventKind {
    KnownTaskEventKind::TurnEnded {
        reason,
        reply_delivered: false,
    }
}

fn intent(call_id: CallId) -> KnownTaskEventKind {
    KnownTaskEventKind::ToolIntent {
        call_id,
        name: "read_file".to_owned(),
        args_summary: "a".to_owned(),
        level: ToolEffect::ReadOnly,
    }
}

#[test]
fn turn_end_card_timed_out_waits_for_you() {
    let events = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(
            3,
            KnownTaskEventKind::CardShown {
                action: "shell".to_owned(),
                card: "c1".to_owned(),
                kind: CardKind::Action,
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
            },
        ),
        ev(4, turn_ended(TurnEndReason::CardTimedOut)),
    ];
    let view = replay(&events).unwrap();
    assert_eq!(view.status, TaskStatus::AwaitingUser);
    assert_eq!(view.detail, Some(TaskDetail::TimedOut));
}

fn check_step_budget_end() {
    let step_events = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(
            3,
            turn_ended(TurnEndReason::OverBudget {
                item: BudgetItem::Steps,
                used: 50_000,
                limit: 50_000,
            }),
        ),
    ];
    let step_view = replay(&step_events).unwrap();
    assert_eq!(step_view.status, TaskStatus::Stuck);
    assert_eq!(
        step_view.detail,
        Some(TaskDetail::OverBudget {
            item: BudgetItem::Steps,
            used: 50_000,
            limit: 50_000,
        })
    );
}

#[test]
fn turn_end_over_budget_matches_usage() {
    let (c1, c2, c3) = (
        CallId::generate(&SystemClock),
        CallId::generate(&SystemClock),
        CallId::generate(&SystemClock),
    );
    let tool_events = vec![
        ev(1, created()),
        ev(
            2,
            KnownTaskEventKind::BudgetLimit {
                item: BudgetItem::ToolCalls,
                limit: 2,
            },
        ),
        ev(3, turn_started_kind()),
        ev(4, intent(c1)),
        ev(5, KnownTaskEventKind::ToolStarted { call_id: c1 }),
        ev(6, intent(c2)),
        ev(7, KnownTaskEventKind::ToolStarted { call_id: c2 }),
        ev(8, intent(c3)),
        ev(
            9,
            turn_ended(TurnEndReason::OverBudget {
                item: BudgetItem::ToolCalls,
                used: 2,
                limit: 2,
            }),
        ),
    ];
    let expected = Some(TaskDetail::OverBudget {
        item: BudgetItem::ToolCalls,
        used: 2,
        limit: 2,
    });
    let by_usage = replay(tool_events.get(..8).unwrap_or(&[])).unwrap();
    let by_end = replay(&tool_events).unwrap();
    assert_eq!(
        (by_usage.status, by_usage.detail),
        (TaskStatus::Stuck, expected.clone())
    );
    assert_eq!(
        (by_end.status, by_end.detail),
        (TaskStatus::Stuck, expected)
    );
    check_step_budget_end();
}

#[test]
fn verification_started_sets_verifying() {
    let criteria = vec![
        Criterion::FileExists {
            path: "a.txt".to_owned(),
        },
        Criterion::FileExists {
            path: "b.txt".to_owned(),
        },
    ];
    let events = vec![
        ev(1, created()),
        ev(
            2,
            KnownTaskEventKind::DoneWhenChange {
                by: Proposer::User,
                change: DoneWhenChange::SetItems { items: criteria },
                before: Vec::new(),
                after: Vec::new(),
                reason: "set".to_owned(),
            },
        ),
        ev(3, turn_started_kind()),
        ev(4, KnownTaskEventKind::VerificationStarted),
        ev(
            5,
            KnownTaskEventKind::CriterionPassed {
                index: 0,
                source: CheckSource::Program,
            },
        ),
        ev(
            6,
            KnownTaskEventKind::CriterionPassed {
                index: 1,
                source: CheckSource::Program,
            },
        ),
    ];
    let while_verifying = replay(events.get(..4).unwrap_or(&[])).unwrap();
    assert_eq!(while_verifying.status, TaskStatus::Running);
    assert_eq!(while_verifying.detail, Some(TaskDetail::Verifying));

    let partial = replay(events.get(..5).unwrap_or(&[])).unwrap();
    assert_eq!(partial.status, TaskStatus::Running);
    assert_eq!(partial.detail, Some(TaskDetail::Verifying));

    let all_done = replay(&events).unwrap();
    assert_eq!(all_done.status, TaskStatus::Completed);
    assert_eq!(all_done.detail, Some(TaskDetail::Accepted));
}

#[test]
fn turn_end_interrupted_and_unreachable_states() {
    let call_id = CallId::generate(&SystemClock);
    let interrupted_unknown = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(3, KnownTaskEventKind::OutcomeUnknown { call_id }),
        ev(4, turn_ended(TurnEndReason::Interrupted)),
    ];
    let view = replay(&interrupted_unknown).unwrap();
    assert_eq!(view.status, TaskStatus::AwaitingUser);
    assert_eq!(view.detail, Some(TaskDetail::ResumePending));

    let interrupted_read = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(3, turn_ended(TurnEndReason::Interrupted)),
    ];
    let auto_view = replay(&interrupted_read).unwrap();
    assert_eq!(auto_view.status, TaskStatus::Running);
    assert_eq!(auto_view.detail, Some(TaskDetail::AutoResuming));

    let unreachable = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(3, KnownTaskEventKind::ModelUnreachable),
        ev(4, turn_ended(TurnEndReason::ModelUnreachable)),
    ];
    let stuck_view = replay(&unreachable).unwrap();
    assert_eq!(stuck_view.status, TaskStatus::Stuck);
    assert_eq!(stuck_view.detail, Some(TaskDetail::ModelUnreachable));
}

#[test]
fn turn_end_stopped_and_cancelled_follow_events() {
    let stopped_with_event = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(3, KnownTaskEventKind::TaskStopped),
        ev(4, turn_ended(TurnEndReason::Stopped)),
    ];
    let stopped_view = replay(&stopped_with_event).unwrap();
    assert_eq!(stopped_view.status, TaskStatus::Stopped);
    assert_eq!(stopped_view.detail, Some(TaskDetail::YouStopped));

    let label_without_event = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(3, turn_ended(TurnEndReason::Stopped)),
    ];
    let still_running = replay(&label_without_event).unwrap();
    assert_eq!(still_running.status, TaskStatus::Running);

    let cancelled_without_event = vec![
        ev(1, created()),
        ev(2, turn_started_kind()),
        ev(3, turn_ended(TurnEndReason::Cancelled)),
    ];
    let running_cancel_label = replay(&cancelled_without_event).unwrap();
    assert_eq!(running_cancel_label.status, TaskStatus::Running);
}

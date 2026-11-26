use anybot_foundation::{EventId, SystemClock, TimestampMs};
use anybot_protocol::{KnownTaskEventKind, RawTaskEventKind, TaskEvent, TaskEventKind, TaskStatus};

use super::*;

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

fn raw_ev(seq: u64) -> TaskEvent {
    TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq,
        at_ms: TimestampMs::from_millis(seq),
        kind: TaskEventKind::Raw(RawTaskEventKind {
            kind: "quantum_flux_gate".to_owned(),
            data: serde_json::Value::Null,
        }),
    }
}

#[test]
fn t_task_events_2() {
    let created = ev(
        1,
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
    );
    let once = replay(std::slice::from_ref(&created)).unwrap();
    let twice = replay(&[created.clone(), created]).unwrap();
    assert_eq!(once, twice);
}

#[test]
fn t_task_events_3() {
    let a = ev(
        1,
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
    );
    let b = ev(2, turn_started_kind());
    let d = ev(
        4,
        KnownTaskEventKind::TurnEnded {
            reason: anybot_protocol::TurnEndReason::Answered,
            reply_delivered: true,
        },
    );
    let err = replay(&[a.clone(), b.clone(), d.clone()]).unwrap_err();
    assert_eq!(err.missing, vec![3]);
    let c = ev(
        3,
        KnownTaskEventKind::ModelAnswer {
            text: "x".to_owned(),
            thinking: None,
            tool_requests: Vec::new(),
            finish: anybot_protocol::FinishReason::Finished,
            native: None,
        },
    );
    let filled = replay(&[a, b, c, d]).unwrap();
    assert_eq!(filled.status, TaskStatus::Completed);
}

#[test]
fn t_task_events_4() {
    let known = [
        ev(
            1,
            KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
                opened_by: anybot_protocol::TaskOpener::You,
                bot: anybot_foundation::BotId::generate(&SystemClock),
                parent: None,
            },
        ),
        ev(3, turn_started_kind()),
    ];
    let mut with_raw = vec![];
    with_raw.extend_from_slice(std::slice::from_ref(&known[0]));
    with_raw.push(raw_ev(2));
    with_raw.extend_from_slice(std::slice::from_ref(&known[1]));
    let known_full = [
        ev(
            1,
            KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
                opened_by: anybot_protocol::TaskOpener::You,
                bot: anybot_foundation::BotId::generate(&SystemClock),
                parent: None,
            },
        ),
        ev(
            2,
            KnownTaskEventKind::ModelAnswer {
                text: "x".to_owned(),
                thinking: None,
                tool_requests: Vec::new(),
                finish: anybot_protocol::FinishReason::Finished,
                native: None,
            },
        ),
        ev(3, turn_started_kind()),
    ];
    let a = replay(&with_raw).unwrap();
    let b = replay(&known_full).unwrap();
    assert_eq!(a, b);
    let names = with_raw.get(1).map(|ev| ev.kind.kind_name());
    assert_eq!(names, Some("quantum_flux_gate".to_owned()));
}

#[test]
fn t_task_events_5() {
    let events = vec![
        ev(
            1,
            KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
                opened_by: anybot_protocol::TaskOpener::You,
                bot: anybot_foundation::BotId::generate(&SystemClock),
                parent: None,
            },
        ),
        ev(2, turn_started_kind()),
        ev(
            3,
            KnownTaskEventKind::TurnEnded {
                reason: anybot_protocol::TurnEndReason::Answered,
                reply_delivered: true,
            },
        ),
    ];
    let mut snapshot = snapshot_at(&events, 2);
    snapshot.format_version = SNAPSHOT_FORMAT_VERSION + 1;
    let view = replay_from(&snapshot, &events).unwrap();
    let full = replay(&events).unwrap();
    assert_eq!(view, full);
}

#[test]
fn t_task_events_6() {
    let events = vec![
        ev(
            1,
            KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
                opened_by: anybot_protocol::TaskOpener::You,
                bot: anybot_foundation::BotId::generate(&SystemClock),
                parent: None,
            },
        ),
        ev(2, turn_started_kind()),
        ev(
            3,
            KnownTaskEventKind::ModelAnswer {
                text: "done, sent".to_owned(),
                thinking: None,
                tool_requests: Vec::new(),
                finish: anybot_protocol::FinishReason::Finished,
                native: None,
            },
        ),
    ];
    let view = replay(&events).unwrap();
    let baseline = replay(events.get(..2).unwrap_or(&[])).unwrap();
    assert_eq!(view, baseline);
    assert_eq!(view.status, TaskStatus::Running);
}

#[test]
fn test_snapshot_round_trips_through_json() {
    let events = vec![
        ev(
            1,
            KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
                opened_by: anybot_protocol::TaskOpener::You,
                bot: anybot_foundation::BotId::generate(&SystemClock),
                parent: None,
            },
        ),
        ev(2, turn_started_kind()),
    ];
    let snapshot = snapshot_at(&events, 1);
    let text = serde_json::to_string(&snapshot).unwrap();
    let back: TaskSnapshot = serde_json::from_str(&text).unwrap();
    assert_eq!(back, snapshot);
    let view = replay_from(&back, &events).unwrap();
    assert_eq!(view, replay(&events).unwrap());
}

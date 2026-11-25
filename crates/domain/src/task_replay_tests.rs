use anybot_foundation::{EventId, SystemClock, TimestampMs};
use anybot_protocol::{KnownTaskEventKind, RawTaskEventKind, TaskEvent, TaskEventKind, TaskStatus};

use super::*;

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
fn t_task_events_2_duplicate_events_apply_once() {
    let created = ev(
        1,
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
    );
    let once = replay(std::slice::from_ref(&created)).unwrap();
    let twice = replay(&[created.clone(), created]).unwrap();
    assert_eq!(once, twice);
}

#[test]
fn t_task_events_3_gap_reports_missing_seq_and_replays_after_fill() {
    let a = ev(
        1,
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
    );
    let b = ev(2, KnownTaskEventKind::TurnStarted);
    let d = ev(
        4,
        KnownTaskEventKind::TurnEnded {
            reply_delivered: true,
        },
    );
    let err = replay(&[a.clone(), b.clone(), d.clone()]).unwrap_err();
    assert_eq!(err.missing, vec![3]);
    let c = ev(
        3,
        KnownTaskEventKind::TurnText {
            text: "x".to_owned(),
        },
    );
    let filled = replay(&[a, b, c, d]).unwrap();
    assert_eq!(filled.status, TaskStatus::Completed);
}

#[test]
fn t_task_events_4_unknown_events_are_skipped_and_kept() {
    let known = [
        ev(
            1,
            KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
            },
        ),
        ev(3, KnownTaskEventKind::TurnStarted),
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
            },
        ),
        ev(
            2,
            KnownTaskEventKind::TurnText {
                text: "x".to_owned(),
            },
        ),
        ev(3, KnownTaskEventKind::TurnStarted),
    ];
    let a = replay(&with_raw).unwrap();
    let b = replay(&known_full).unwrap();
    assert_eq!(a, b);
    let names = with_raw.get(1).map(|ev| ev.kind.kind_name());
    assert_eq!(names, Some("quantum_flux_gate".to_owned()));
}

#[test]
fn t_task_events_5_old_snapshot_version_replays_from_zero() {
    let events = vec![
        ev(
            1,
            KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
            },
        ),
        ev(2, KnownTaskEventKind::TurnStarted),
        ev(
            3,
            KnownTaskEventKind::TurnEnded {
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
fn t_task_events_6_completion_claim_text_changes_nothing() {
    let events = vec![
        ev(
            1,
            KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
            },
        ),
        ev(2, KnownTaskEventKind::TurnStarted),
        ev(
            3,
            KnownTaskEventKind::TurnText {
                text: "done, sent".to_owned(),
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
            },
        ),
        ev(2, KnownTaskEventKind::TurnStarted),
    ];
    let snapshot = snapshot_at(&events, 1);
    let text = serde_json::to_string(&snapshot).unwrap();
    let back: TaskSnapshot = serde_json::from_str(&text).unwrap();
    assert_eq!(back, snapshot);
    let view = replay_from(&back, &events).unwrap();
    assert_eq!(view, replay(&events).unwrap());
}

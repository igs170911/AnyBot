use super::*;
use anybot_foundation::{CallId, EventId, SystemClock};

fn event(seq: u64, kind: KnownTaskEventKind) -> TaskEvent {
    TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq,
        at_ms: TimestampMs::from_millis(1000),
        kind: TaskEventKind::Known(kind),
    }
}

#[test]
fn test_task_events_known_kind_round_trips() {
    let e = event(
        1,
        KnownTaskEventKind::ToolIntent {
            call_id: CallId::generate(&SystemClock),
            name: "send".to_owned(),
            args_summary: "a".to_owned(),
            level: ToolEffect::Irreversible,
        },
    );
    let text = serde_json::to_string(&e).unwrap();
    let back: TaskEvent = serde_json::from_str(&text).unwrap();
    assert_eq!(back, e);
}

#[test]
fn test_task_events_unknown_kind_is_kept() {
    let e = TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq: 2,
        at_ms: TimestampMs::from_millis(1),
        kind: TaskEventKind::Raw(RawTaskEventKind {
            kind: "time_machine_started".to_owned(),
            data: serde_json::json!({ "tz": "UTC" }),
        }),
    };
    let text = serde_json::to_string(&e).unwrap();
    let back: TaskEvent = serde_json::from_str(&text).unwrap();
    assert_eq!(back.kind.kind_name(), "time_machine_started");
    assert!(back.known().is_none());
}

#[test]
fn test_task_events_effect_level_defaults_to_irreversible() {
    assert_eq!(effect_level(None), ToolEffect::Irreversible);
    assert_eq!(
        effect_level(Some(ToolEffect::ReadOnly)),
        ToolEffect::ReadOnly
    );
}

#[test]
fn test_task_events_done_when_change_round_trips() {
    let e = event(
        3,
        KnownTaskEventKind::DoneWhenChange {
            by: Proposer::Bot,
            change: DoneWhenChange::RemoveItem { index: 1 },
            before: vec![Criterion::FileExists {
                path: "out.txt".to_owned(),
            }],
            after: vec![],
            reason: "too strict".to_owned(),
        },
    );
    let text = serde_json::to_string(&e).unwrap();
    let back: TaskEvent = serde_json::from_str(&text).unwrap();
    assert_eq!(back, e);
}

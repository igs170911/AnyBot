use anybot_foundation::{EventId, SystemClock, TimestampMs};
use anybot_protocol::{KnownTaskEventKind, TaskEvent, TaskEventKind};

use super::{ContextCollapseReason, ContextCollapsed, collapsed_kind, parse_collapsed};
use crate::task_replay::replay;

fn event(seq: u64, kind: TaskEventKind) -> TaskEvent {
    TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq,
        at_ms: TimestampMs::from_millis(seq),
        kind,
    }
}

#[test]
fn test_context_events_roundtrip() {
    let collapsed = ContextCollapsed {
        ids: vec!["a".to_owned(), "b".to_owned()],
        reason: ContextCollapseReason::Accumulated,
    };
    let kind = collapsed_kind(&collapsed);
    assert_eq!(parse_collapsed(&kind), Some(collapsed));
}

#[test]
fn test_context_events_parse_ignores_other_raw_kinds() {
    let kind = TaskEventKind::Raw(anybot_protocol::RawTaskEventKind {
        kind: "something_new".to_owned(),
        data: serde_json::Value::Null,
    });
    assert_eq!(parse_collapsed(&kind), None);
}

#[test]
fn test_context_events_state_skips_collapse_event() {
    let base = vec![
        event(
            1,
            TaskEventKind::Known(KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
            }),
        ),
        event(2, TaskEventKind::Known(KnownTaskEventKind::TurnStarted)),
    ];
    let with_raw = {
        let mut v = base.clone();
        v.push(event(
            3,
            collapsed_kind(&ContextCollapsed {
                ids: vec!["x".to_owned()],
                reason: ContextCollapseReason::Compact,
            }),
        ));
        v
    };
    let a = replay(&base).unwrap();
    let b = replay(&with_raw).unwrap();
    assert_eq!(a, b);
}

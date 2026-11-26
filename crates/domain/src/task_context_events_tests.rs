use anybot_foundation::{EventId, SystemClock, TimestampMs};
use anybot_protocol::{KnownTaskEventKind, TaskEvent, TaskEventKind};

use super::{
    CONTEXT_COLLAPSED_KIND, ContextCollapseReason, ContextCollapsed, collapsed_kind,
    memory_dropped_kind, parse_collapsed, parse_memory_dropped,
};
use crate::task_replay::replay;

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

fn event(seq: u64, kind: TaskEventKind) -> TaskEvent {
    TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq,
        at_ms: TimestampMs::from_millis(seq),
        kind,
    }
}

#[test]
fn collapsed_event_with_replace_all_roundtrips() {
    let collapsed = ContextCollapsed {
        ids: vec!["a".to_owned()],
        reason: ContextCollapseReason::ReplaceAll,
    };
    let kind = collapsed_kind(&collapsed);
    assert_eq!(parse_collapsed(&kind), Ok(Some(collapsed)));
}

#[test]
fn memory_dropped_event_roundtrips() {
    let ids = vec!["m1".to_owned(), "m2".to_owned()];
    let kind = memory_dropped_kind(&ids);
    assert_eq!(parse_memory_dropped(&kind), Ok(Some(ids)));
}

#[test]
fn malformed_collapsed_event_reports_an_error() {
    let kind = TaskEventKind::Raw(anybot_protocol::RawTaskEventKind {
        kind: CONTEXT_COLLAPSED_KIND.to_owned(),
        data: serde_json::Value::Null,
    });
    assert!(parse_collapsed(&kind).is_err());
}

#[test]
fn test_context_events_roundtrip() {
    let collapsed = ContextCollapsed {
        ids: vec!["a".to_owned(), "b".to_owned()],
        reason: ContextCollapseReason::Accumulated,
    };
    let kind = collapsed_kind(&collapsed);
    assert_eq!(parse_collapsed(&kind), Ok(Some(collapsed)));
}

#[test]
fn test_context_events_parse_ignores_other_raw_kinds() {
    let kind = TaskEventKind::Raw(anybot_protocol::RawTaskEventKind {
        kind: "something_new".to_owned(),
        data: serde_json::Value::Null,
    });
    assert_eq!(parse_collapsed(&kind), Ok(None));
}

#[test]
fn test_context_events_state_skips_collapse_event() {
    let base = vec![
        event(
            1,
            TaskEventKind::Known(KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
                opened_by: anybot_protocol::TaskOpener::You,
                bot: anybot_foundation::BotId::generate(&SystemClock),
                parent: None,
            }),
        ),
        event(2, TaskEventKind::Known(turn_started_kind())),
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

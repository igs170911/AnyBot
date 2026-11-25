use super::*;
use anybot_foundation::{BlobId, CallId, CardId, EventId, EvidenceId, SystemClock, TaskId, TurnId};
use proptest::prelude::*;

const TASK: &str = "018f3b7e-2b3f-7000-8000-000000000000";

fn text_kind() -> EventKind {
    EventKind::TurnText {
        turn_id: TurnId::generate(&SystemClock),
        text: "你好".to_owned(),
    }
}

fn sample_event(seq: u64, kind: EventKind) -> RunEvent {
    RunEvent::new(&SystemClock, TaskId::parse(TASK).unwrap(), seq, kind)
}

#[test]
fn test_events_roundtrip_keeps_content() {
    let e = sample_event(3, text_kind());
    let json = serde_json::to_string(&e).unwrap();
    let back: RunEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back, e);
    assert_eq!(e.seq, 3);
}

#[test]
fn test_events_envelope_fields_serialized() {
    let e = sample_event(
        9,
        EventKind::TurnEnded {
            turn_id: TurnId::generate(&SystemClock),
        },
    );
    let value: serde_json::Value = serde_json::to_value(&e).unwrap();
    assert_eq!(value.get("seq").unwrap().as_u64(), Some(9));
    assert_eq!(value.get("protocol_version").unwrap().as_u64(), Some(1));
    assert_eq!(
        value.get("kind").unwrap().get("kind").unwrap().as_str(),
        Some("turn_ended")
    );
    assert!(
        value
            .get("kind")
            .unwrap()
            .get("data")
            .unwrap()
            .get("turn_id")
            .unwrap()
            .as_str()
            .is_some_and(|s| EventId::parse(s).is_ok())
    );
    assert!(value.get("at_ms").unwrap().is_number());
}

#[test]
fn t_agent_ext_protocol_5() {
    let mut e = sample_event(1, text_kind());
    assert!(e.accept().is_ok());
    e.protocol_version = 0;
    assert!(e.accept().is_err());
}

#[test]
fn test_events_all_kinds_roundtrip() {
    let turn = TurnId::generate(&SystemClock);
    let call = CallId::generate(&SystemClock);
    let card = CardId::generate(&SystemClock);
    let ev = EvidenceId::generate(&SystemClock);
    let blob = BlobId::generate(&SystemClock);
    let kinds = vec![
        EventKind::TurnStarted { turn_id: turn },
        text_kind(),
        EventKind::TurnThinking {
            turn_id: turn,
            text: "想".to_owned(),
        },
        EventKind::TurnEnded { turn_id: turn },
        EventKind::ToolCallStarted {
            turn_id: turn,
            call_id: call,
            name: "ls".to_owned(),
            args: serde_json::json!({"p": "."}),
        },
        EventKind::ToolCallResult {
            turn_id: turn,
            call_id: call,
            ok: true,
            output: serde_json::json!("done"),
        },
        EventKind::ApprovalCard {
            turn_id: turn,
            card_id: card,
            summary: "s".to_owned(),
            options: vec!["yes".to_owned()],
        },
        EventKind::EvidenceAdded {
            turn_id: turn,
            evidence_id: ev,
            summary: "e".to_owned(),
            blob_id: blob,
        },
        EventKind::TaskStatusChanged {
            status: TaskStatus::Running,
        },
        EventKind::RunError {
            message: "m".to_owned(),
        },
    ];
    for kind in kinds {
        let e = sample_event(1, kind.clone());
        let json = serde_json::to_string(&e).unwrap();
        let back: RunEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, kind);
    }
}

fn kind_strategy() -> BoxedStrategy<EventKind> {
    let turn = Just(TurnId::generate(&SystemClock)).boxed();
    let text = proptest::string::string_regex("[a-z]{0,8}")
        .unwrap()
        .boxed();
    prop_oneof![
        turn.clone()
            .prop_map(|t| EventKind::TurnStarted { turn_id: t }),
        (turn.clone(), text.clone()).prop_map(|(t, s)| EventKind::TurnText {
            turn_id: t,
            text: s.clone(),
        }),
        (turn.clone(), text.clone()).prop_map(|(t, s)| EventKind::TurnThinking {
            turn_id: t,
            text: s.clone(),
        }),
        turn.clone()
            .prop_map(|t| EventKind::TurnEnded { turn_id: t }),
        (
            turn.clone(),
            proptest::string::string_regex("[a-z]{1,6}")
                .unwrap()
                .boxed()
        )
            .prop_map(|(t, n)| {
                EventKind::ToolCallStarted {
                    turn_id: t,
                    call_id: CallId::generate(&SystemClock),
                    name: n.clone(),
                    args: serde_json::Value::Null,
                }
            }),
        turn.clone().prop_map(|t| EventKind::ToolCallResult {
            turn_id: t,
            call_id: CallId::generate(&SystemClock),
            ok: true,
            output: serde_json::Value::Null,
        }),
        (turn.clone(), text.clone()).prop_map(|(t, s)| EventKind::ApprovalCard {
            turn_id: t,
            card_id: CardId::generate(&SystemClock),
            summary: s.clone(),
            options: vec![],
        }),
        (turn.clone(), text.clone()).prop_map(|(t, s)| EventKind::EvidenceAdded {
            turn_id: t,
            evidence_id: EvidenceId::generate(&SystemClock),
            summary: s.clone(),
            blob_id: BlobId::generate(&SystemClock),
        }),
        Just(EventKind::TaskStatusChanged {
            status: TaskStatus::Completed,
        }),
        text.prop_map(|s| EventKind::RunError { message: s.clone() }),
    ]
    .boxed()
}

proptest! {
    #[test]
    fn prop_events_roundtrip(kind in kind_strategy()) {
        let e = RunEvent::new(&SystemClock, TaskId::generate(&SystemClock), 7, kind);
        let json = serde_json::to_string(&e).unwrap();
        let back: RunEvent = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(back, e);
    }
}

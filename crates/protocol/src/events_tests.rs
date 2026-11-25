use super::*;
use anybot_foundation::{BlobId, CallId, CardId, EvidenceId, SystemClock, TaskId, TurnId};

const TURN: &str = "018f3b7e-2b3f-7000-8000-000000000000";
const TASK: &str = "019f3b7e-2b3f-7000-8000-000000000001";

fn turn() -> TurnId {
    TurnId::parse(TURN).unwrap()
}

fn text_kind() -> EventKind {
    EventKind::Known(KnownEventKind::TurnText {
        text: "\u{4f60}\u{597d}".to_owned(),
    })
}

fn sample_event(kind: EventKind) -> RunEvent {
    RunEvent::new(&SystemClock, turn(), kind)
}

#[test]
fn test_events_roundtrip_keeps_content() {
    let e = sample_event(text_kind());
    let json = serde_json::to_string(&e).unwrap();
    let back: RunEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back, e);
}

#[test]
fn test_events_envelope_has_turn_id_and_no_task_id_or_seq() {
    let e = sample_event(text_kind());
    let value: serde_json::Value = serde_json::to_value(&e).unwrap();
    assert_eq!(value.get("protocol_version").unwrap().as_u64(), Some(1));
    assert_eq!(
        value.get("turn_id").unwrap().as_str(),
        Some("018f3b7e-2b3f-7000-8000-000000000000")
    );
    assert!(value.get("event_id").unwrap().as_str().is_some());
    assert!(value.get("at_ms").unwrap().is_number());
    assert!(value.get("seq").is_none());
    assert!(value.get("task_id").is_none());
    assert_eq!(
        value.get("kind").unwrap().get("kind").unwrap().as_str(),
        Some("turn_text")
    );
}

#[test]
fn test_events_recorded_adds_seq_keeps_envelope() {
    let e = sample_event(text_kind());
    let r = e.recorded(9);
    let value: serde_json::Value = serde_json::to_value(&r).unwrap();
    assert_eq!(value.get("seq").unwrap().as_u64(), Some(9));
    assert_eq!(value.get("turn_id").unwrap().as_str(), Some(TURN));
    assert_eq!(value.get("protocol_version").unwrap().as_u64(), Some(1));
    let back: RecordedEvent = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(back, r);
}

#[test]
fn test_events_task_status_changed_carries_task_id() {
    let kind = EventKind::Known(KnownEventKind::TaskStatusChanged {
        task_id: TaskId::parse(TASK).unwrap(),
        status: TaskStatus::Completed,
        detail: Some(TaskDetail::Delivered),
    });
    let e = sample_event(kind);
    let value: serde_json::Value = serde_json::to_value(&e).unwrap();
    assert_eq!(
        value
            .get("kind")
            .unwrap()
            .get("data")
            .unwrap()
            .get("task_id")
            .unwrap()
            .as_str(),
        Some(TASK)
    );
    assert_eq!(
        value
            .get("kind")
            .unwrap()
            .get("data")
            .unwrap()
            .get("detail")
            .unwrap()
            .get("detail")
            .unwrap()
            .as_str(),
        Some("delivered")
    );
    let plain = EventKind::Known(KnownEventKind::TaskStatusChanged {
        task_id: TaskId::parse(TASK).unwrap(),
        status: TaskStatus::Running,
        detail: None,
    });
    let value: serde_json::Value = serde_json::to_value(sample_event(plain)).unwrap();
    assert!(
        value
            .get("kind")
            .unwrap()
            .get("data")
            .unwrap()
            .get("detail")
            .is_none()
    );
}

#[test]
fn test_events_unknown_kind_keeps_raw_and_batch_parses() {
    let json = r#"{"event_id":"019f3b7e-2b3f-7000-8000-000000000002","turn_id":"018f3b7e-2b3f-7000-8000-000000000000","at_ms":1700000000000,"protocol_version":1,"kind":{"kind":"mystery_kind","data":{"stuff":[1,2]}}}"#;
    let batch = format!(
        "[{json},{}]",
        serde_json::to_string(&sample_event(text_kind())).unwrap()
    );
    let events: Vec<RunEvent> = serde_json::from_str(&batch).unwrap();
    assert_eq!(events.len(), 2);
    let first = events.first().unwrap();
    assert_eq!(
        first.kind,
        EventKind::Raw(RawEventKind {
            kind: "mystery_kind".to_owned(),
            data: serde_json::json!({"stuff": [1, 2]}),
        })
    );
    let back = serde_json::to_string(&first.kind).unwrap();
    let reparsed: EventKind = serde_json::from_str(&back).unwrap();
    assert_eq!(reparsed, first.kind);
    assert_eq!(first.kind.kind_name(), "mystery_kind");
}

#[test]
fn test_events_accept_with_system_clock_accepts_generated_event() {
    let e = sample_event(text_kind());
    assert!(e.accept(&SystemClock).is_ok());
}

#[test]
fn test_events_accept_rejects_unknown_versions() {
    let mut e = sample_event(text_kind());
    e.protocol_version = 0;
    assert!(e.accept(&SystemClock).is_err());
    let mut e = sample_event(text_kind());
    e.protocol_version = PROTOCOL_VERSION + 1;
    assert!(e.accept(&SystemClock).is_err());
}

#[test]
fn run_error_carries_code_and_params_not_message() {
    let mut e = sample_event(text_kind());
    e.protocol_version = PROTOCOL_VERSION + 2;
    let kind = KnownEventKind::RunError {
        code: RUN_ERROR_CODE_PROTOCOL_VERSION.to_owned(),
        params: MessageParams::from_pairs([
            (
                "version".to_owned(),
                serde_json::Value::from(e.protocol_version),
            ),
            (
                "supported".to_owned(),
                serde_json::Value::from(PROTOCOL_VERSION),
            ),
        ]),
    };
    let value = serde_json::to_value(EventKind::Known(kind)).unwrap();
    assert_eq!(value.get("kind").unwrap().as_str(), Some("run_error"));
    let data = value.get("data").unwrap();
    assert_eq!(
        data.get("code").unwrap().as_str(),
        Some(RUN_ERROR_CODE_PROTOCOL_VERSION)
    );
    assert_eq!(
        data.get("params").unwrap().get("version").unwrap().as_u64(),
        Some(3)
    );
    assert_eq!(
        data.get("params")
            .unwrap()
            .get("supported")
            .unwrap()
            .as_u64(),
        Some(1)
    );
    assert!(data.get("message").is_none());
    let empty = EventKind::Known(KnownEventKind::RunError {
        code: RUN_ERROR_CODE_UNSUPPORTED_EVENT.to_owned(),
        params: MessageParams::default(),
    });
    let json = serde_json::to_string(&empty).unwrap();
    assert!(!json.contains("\"params\""));
}

#[test]
fn test_events_all_kinds_roundtrip() {
    let call = CallId::generate(&SystemClock);
    let card = CardId::generate(&SystemClock);
    let ev = EvidenceId::generate(&SystemClock);
    let blob = BlobId::generate(&SystemClock);
    let task = TaskId::parse(TASK).unwrap();
    let kinds = vec![
        EventKind::Known(KnownEventKind::TurnStarted {}),
        text_kind(),
        EventKind::Known(KnownEventKind::TurnThinking {
            text: "\u{60f3}".to_owned(),
        }),
        EventKind::Known(KnownEventKind::TurnEnded {}),
        EventKind::Known(KnownEventKind::ToolCallStarted {
            call_id: call,
            name: "ls".to_owned(),
            args: serde_json::json!({"p": "."}),
        }),
        EventKind::Known(KnownEventKind::ToolCallResult {
            call_id: call,
            ok: true,
            output: serde_json::json!("done"),
        }),
        EventKind::Known(KnownEventKind::ApprovalCard {
            card_id: card,
            summary: "s".to_owned(),
            options: vec!["yes".to_owned()],
        }),
        EventKind::Known(KnownEventKind::EvidenceAdded {
            evidence_id: ev,
            summary: "e".to_owned(),
            blob_id: blob,
        }),
        EventKind::Known(KnownEventKind::TaskStatusChanged {
            task_id: task,
            status: TaskStatus::Running,
            detail: None,
        }),
        EventKind::Known(KnownEventKind::RunError {
            code: RUN_ERROR_CODE_PROTOCOL_VERSION.to_owned(),
            params: MessageParams::from_pairs([
                ("version".to_owned(), serde_json::json!(9)),
                ("supported".to_owned(), serde_json::json!(PROTOCOL_VERSION)),
            ]),
        }),
    ];
    for kind in kinds {
        let e = sample_event(kind.clone());
        let json = serde_json::to_string(&e).unwrap();
        let back: RunEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, kind);
    }
}

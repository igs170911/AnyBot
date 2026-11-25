use super::*;
use anybot_foundation::{BlobId, CallId, CardId, EvidenceId, SystemClock, TurnId};
use proptest::prelude::*;

fn kind_strategy() -> BoxedStrategy<EventKind> {
    let text = proptest::string::string_regex("[a-z]{0,8}")
        .unwrap()
        .boxed();
    prop_oneof![
        Just(EventKind::Known(KnownEventKind::TurnStarted {})),
        text.clone()
            .prop_map(|s| EventKind::Known(KnownEventKind::TurnText { text: s.clone() })),
        text.clone()
            .prop_map(|s| EventKind::Known(KnownEventKind::TurnThinking { text: s.clone() })),
        Just(EventKind::Known(KnownEventKind::TurnEnded {})),
        (text.clone(), Just(CallId::generate(&SystemClock)),).prop_map(|(n, c)| {
            EventKind::Known(KnownEventKind::ToolCallStarted {
                call_id: c,
                name: n.clone(),
                args: serde_json::Value::Null,
            })
        }),
        Just(EventKind::Known(KnownEventKind::ToolCallResult {
            call_id: CallId::generate(&SystemClock),
            ok: true,
            output: serde_json::Value::Null,
        })),
        (text.clone(), Just(CardId::generate(&SystemClock))).prop_map(|(s, c)| {
            EventKind::Known(KnownEventKind::ApprovalCard {
                card_id: c,
                summary: s.clone(),
                options: vec![],
            })
        }),
        (
            text.clone(),
            Just(EvidenceId::generate(&SystemClock)),
            Just(BlobId::generate(&SystemClock)),
        )
            .prop_map(|(s, e, b)| {
                EventKind::Known(KnownEventKind::EvidenceAdded {
                    evidence_id: e,
                    summary: s.clone(),
                    blob_id: b,
                })
            }),
        text.clone()
            .prop_map(|s| EventKind::Known(KnownEventKind::RunError {
                code: RUN_ERROR_CODE_UNSUPPORTED_EVENT.to_owned(),
                params: MessageParams::from_pairs([(
                    "event".to_owned(),
                    serde_json::json!(s.clone())
                ),]),
            })),
    ]
    .boxed()
}

proptest! {
    #[test]
    fn prop_events_roundtrip(kind in kind_strategy()) {
        let e = RunEvent::new(&SystemClock, TurnId::generate(&SystemClock), kind);
        let json = serde_json::to_string(&e).unwrap();
        let back: RunEvent = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(back, e);
    }
}

use super::*;
use crate::{CardButton, CardKind, ReasonPayload};
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

#[test]
fn test_task_events_card_shown_round_trips() {
    let e = event(
        4,
        KnownTaskEventKind::CardShown {
            action: "command git push on bot-computer".to_owned(),
            card: "abc123def456".to_owned(),
            kind: CardKind::Action,
            buttons: vec![
                CardButton::AllowOnce,
                CardButton::AllowConversation,
                CardButton::AlwaysAllow,
                CardButton::Deny,
            ],
            reason: ReasonPayload {
                code: "card-run-command".to_owned(),
                params: vec![("command".to_owned(), "git push".to_owned())],
            },
            always_label: Some(ReasonPayload {
                code: "always-run-command-bot-computer".to_owned(),
                params: vec![("words".to_owned(), "git push".to_owned())],
            }),
            notes: vec![ReasonPayload {
                code: "card-note-every-time".to_owned(),
                params: Vec::new(),
            }],
            classes: vec!["publish".to_owned()],
        },
    );
    let text = serde_json::to_string(&e).unwrap();
    let back: TaskEvent = serde_json::from_str(&text).unwrap();
    assert_eq!(back, e);
}

#[test]
fn test_task_events_claim_checked_round_trips() {
    let e = event(
        5,
        KnownTaskEventKind::ClaimChecked {
            covers_claim: true,
            verified: false,
            reasons: vec![ReasonPayload {
                code: "claim-chat-count".to_owned(),
                params: vec![
                    ("said".to_owned(), "2".to_owned()),
                    ("found".to_owned(), "1".to_owned()),
                ],
            }],
        },
    );
    let text = serde_json::to_string(&e).unwrap();
    let back: TaskEvent = serde_json::from_str(&text).unwrap();
    assert_eq!(back, e);
}

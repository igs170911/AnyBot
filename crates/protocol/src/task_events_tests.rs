use super::*;
use crate::{CardButton, CardKind, QuestionAnswer, QuestionItem, QuestionKind, ReasonPayload};
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

fn round_trip<T>(value: &T) -> T
where
    T: Serialize + serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
{
    let text = serde_json::to_string(value).unwrap();
    let back: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&back, value);
    back
}

fn check_resume_and_referee_round_trip() {
    check_resume_card_round_trip();
    check_legacy_card_decodes();
    check_referee_round_trip();
}

fn check_resume_card_round_trip() {
    let resume_card = KnownTaskEventKind::CardShown {
        action: "resume".to_owned(),
        card: "c-resume".to_owned(),
        kind: CardKind::Resume,
        resume: Some(Box::new(crate::ResumeCard {
            maybe_done: vec![crate::ResumeCardStep {
                call_id: CallId::generate(&SystemClock),
                tool: "shell".to_owned(),
                args_summary: "make".to_owned(),
            }],
            queued_messages: vec!["later".to_owned()],
        })),
        buttons: vec![CardButton::AllowOnce, CardButton::Deny],
        reason: ReasonPayload {
            code: "card-run-command".to_owned(),
            params: Vec::new(),
        },
        always_label: None,
        notes: Vec::new(),
        classes: Vec::new(),
    };
    round_trip(&event(12, resume_card));
}

fn check_legacy_card_decodes() {
    let legacy_card = serde_json::json!({
        "kind": "card_shown",
        "data": {
            "action": "edit_file",
            "card": "c-old",
            "buttons": ["allow_once", "deny"],
            "reason": { "code": "card-run-command", "params": [] },
            "always_label": null,
            "notes": [],
            "classes": []
        }
    });
    let decoded: KnownTaskEventKind = serde_json::from_value(legacy_card).unwrap();
    assert_eq!(
        decoded,
        KnownTaskEventKind::CardShown {
            action: "edit_file".to_owned(),
            card: "c-old".to_owned(),
            kind: CardKind::Action,
            resume: None,
            buttons: vec![CardButton::AllowOnce, CardButton::Deny],
            reason: ReasonPayload {
                code: "card-run-command".to_owned(),
                params: Vec::new(),
            },
            always_label: None,
            notes: Vec::new(),
            classes: Vec::new(),
        }
    );
}

fn check_referee_round_trip() {
    let req = crate::RefereeRequest {
        stream: crate::StreamSelector::Bot {
            id: anybot_foundation::BotId::generate(&SystemClock).as_str(),
        },
        task_id: "t1".to_owned(),
        action: crate::RefereeAction::Wake,
        card_id: None,
        answer: None,
        criteria: None,
        reason: Some(crate::WakeReason::Resume),
    };
    round_trip(&req);
    assert_eq!(crate::WakeReason::Resume.as_str(), "resume");
    assert_eq!(crate::WakeReason::Verification.as_str(), "verification");
}

#[test]
fn new_kinds_round_trip() {
    let device = anybot_foundation::DeviceId::generate(&SystemClock);
    assert!(!crate::TurnInitiator::You { device }.is_unattended());
    assert!(crate::TurnInitiator::Verification.is_unattended());
    round_trip(&crate::TurnInitiator::Verification);

    for reason in [
        crate::TurnEndReason::Stopped,
        crate::TurnEndReason::Cancelled,
        crate::TurnEndReason::OverBudget {
            item: BudgetItem::ToolCalls,
            used: 2,
            limit: 2,
        },
        crate::TurnEndReason::OverBudget {
            item: BudgetItem::Steps,
            used: 50_000,
            limit: 50_000,
        },
        crate::TurnEndReason::CardTimedOut,
        crate::TurnEndReason::ModelUnreachable,
        crate::TurnEndReason::Interrupted,
    ] {
        round_trip(&event(
            10,
            KnownTaskEventKind::TurnEnded {
                reason,
                reply_delivered: false,
            },
        ));
    }

    let ver = event(11, KnownTaskEventKind::VerificationStarted);
    assert_eq!(ver.kind.kind_name(), "verification_started");
    round_trip(&ver);
    round_trip(&crate::ModelChoice {
        model: crate::ModelName {
            provider: "OpenAi".to_owned(),
            base_url: "http://127.0.0.1:9/v1".to_owned(),
            model: "m2".to_owned(),
        },
        reason: "model-server".to_owned(),
        fail_code: "model-server".to_owned(),
    });
    check_resume_and_referee_round_trip();
}

#[test]
fn test_task_events_card_shown_round_trips() {
    let e = event(
        4,
        KnownTaskEventKind::CardShown {
            action: "command git push on bot-computer".to_owned(),
            card: "abc123def456".to_owned(),
            kind: CardKind::Action,
            resume: None,
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

#[test]
fn test_task_events_question_round_trips() {
    let shown = event(
        6,
        KnownTaskEventKind::QuestionShown {
            card: "q1".to_owned(),
            questions: vec![QuestionItem {
                title: "Pick one".to_owned(),
                kind: QuestionKind::Single,
                options: vec!["A".to_owned(), "B".to_owned()],
                prefill: None,
            }],
        },
    );
    let answered = event(
        7,
        KnownTaskEventKind::QuestionAnswered {
            card: "q1".to_owned(),
            answer: QuestionAnswer::Skipped,
        },
    );
    for item in [shown, answered] {
        let text = serde_json::to_string(&item).unwrap();
        let back: TaskEvent = serde_json::from_str(&text).unwrap();
        assert_eq!(back, item);
    }
}

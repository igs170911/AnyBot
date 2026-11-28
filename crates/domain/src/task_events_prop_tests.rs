use proptest::prelude::*;

use anybot_foundation::{CallId, EventId, SystemClock, TimestampMs};
use anybot_protocol::{
    BudgetItem, CardAnswer, Criterion, DeviceReportKind, DoneWhenChange, KnownTaskEventKind,
    Proposer, TaskEvent, TaskEventKind,
};

use super::{replay, replay_from, snapshot_at};

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

fn criterion_strategy() -> BoxedStrategy<Criterion> {
    prop_oneof![
        "[a-z]{1,6}".prop_map(|p| Criterion::FileExists { path: p }),
        "([a-z]{1,6}, ){2}".prop_map(|s| {
            let mut parts = s.split(", ");
            let path = parts.next().unwrap_or("a").to_owned();
            let needle = parts.next().unwrap_or("b").trim_end().to_owned();
            Criterion::FileContains { path, needle }
        }),
        "[a-z]{1,6}".prop_map(|t| Criterion::ModelJudgement { text: t }),
        Just(Criterion::SentCount { count: 2 }),
    ]
    .boxed()
}

fn lifecycle_turn_events() -> BoxedStrategy<KnownTaskEventKind> {
    prop_oneof![
        "[a-z]{1,8}".prop_map(|goal| KnownTaskEventKind::TaskCreated {
            goal,
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        }),
        Just(turn_started_kind()),
        (0usize..2usize).prop_map(|v| KnownTaskEventKind::TurnEnded {
            reason: anybot_protocol::TurnEndReason::Answered,
            reply_delivered: v == 1,
        }),
        "[a-z]{1,8}".prop_map(|t| KnownTaskEventKind::ModelAnswer {
            text: t,
            thinking: None,
            tool_requests: Vec::new(),
            finish: anybot_protocol::FinishReason::Finished,
            native: None
        }),
        Just(KnownTaskEventKind::TaskStopped),
        "[a-z]{1,6}".prop_map(|reason| KnownTaskEventKind::TaskCancelled { reason }),
        Just(KnownTaskEventKind::Wake {
            reason: "w".to_owned(),
        }),
    ]
    .boxed()
}

fn card_goal_events() -> BoxedStrategy<KnownTaskEventKind> {
    prop_oneof![
        "[a-z]{1,4}".prop_map(|a| KnownTaskEventKind::CardShown {
            action: a.clone(),
            card: a,
            kind: anybot_protocol::CardKind::Action,
            resume: None,
        }),
        Just(CardAnswer::AllowOnce).prop_map(|a| KnownTaskEventKind::CardAnswered { answer: a }),
        Just(CardAnswer::Deny).prop_map(|a| KnownTaskEventKind::CardAnswered { answer: a }),
        (0usize..4usize, criterion_strategy()).prop_map(|(index, item)| {
            KnownTaskEventKind::DoneWhenChange {
                by: Proposer::Bot,
                change: DoneWhenChange::AddItem { item },
                before: vec![],
                after: vec![added_marker(index)],
                reason: "r".to_owned(),
            }
        }),
        (0usize..3usize, Just(true)).prop_map(|(index, ok)| {
            if ok {
                KnownTaskEventKind::CriterionPassed {
                    index,
                    source: anybot_protocol::CheckSource::Program,
                }
            } else {
                KnownTaskEventKind::CriterionFailed {
                    index,
                    source: anybot_protocol::CheckSource::Program,
                    observed: "o".to_owned(),
                }
            }
        }),
        Just(KnownTaskEventKind::AcceptanceUnavailable),
        (0usize..2usize, 0usize..2usize).prop_map(|(c, v)| KnownTaskEventKind::ClaimChecked {
            covers_claim: c == 1,
            verified: v == 1,
        }),
    ]
    .boxed()
}

fn budget_tool_events() -> BoxedStrategy<KnownTaskEventKind> {
    prop_oneof![
        (Just(BudgetItem::Tokens), 0u64..1200u64)
            .prop_map(|(item, delta)| KnownTaskEventKind::BudgetUsage { item, delta }),
        (Just(BudgetItem::Tokens), 0u64..1200u64)
            .prop_map(|(item, limit)| KnownTaskEventKind::BudgetLimit { item, limit }),
        Just(KnownTaskEventKind::ToolStarted {
            call_id: CallId::generate(&SystemClock),
        }),
        Just(KnownTaskEventKind::ActionVerified {
            evidence_ids: Vec::new(),
            call_id: CallId::generate(&SystemClock),
            source: anybot_protocol::CheckSource::Program,
        }),
        Just(KnownTaskEventKind::OutcomeUnknown {
            call_id: CallId::generate(&SystemClock),
        }),
        Just(DeviceReportKind::Shown).prop_map(|kind| KnownTaskEventKind::DeviceReport {
            kind,
            file: "f".to_owned(),
        }),
        Just(DeviceReportKind::ThumbsUp).prop_map(|kind| KnownTaskEventKind::DeviceReport {
            kind,
            file: "f".to_owned(),
        }),
    ]
    .boxed()
}

fn kind_strategy() -> BoxedStrategy<KnownTaskEventKind> {
    prop_oneof![
        lifecycle_turn_events(),
        card_goal_events(),
        budget_tool_events()
    ]
    .boxed()
}

fn added_marker(_index: usize) -> Criterion {
    Criterion::FileExists {
        path: "a".to_owned(),
    }
}

fn events_strategy() -> BoxedStrategy<(Vec<KnownTaskEventKind>, usize)> {
    (1usize..40usize)
        .prop_flat_map(|len| {
            prop::collection::vec(kind_strategy(), len).prop_map(move |kinds| {
                let split = kinds.len() / 2;
                (kinds, split)
            })
        })
        .boxed()
}

fn materialize(kinds: &[KnownTaskEventKind]) -> Vec<TaskEvent> {
    kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            let next = i.saturating_add(1);
            let at = u64::try_from(next).unwrap_or(0);
            TaskEvent {
                event_id: EventId::generate(&SystemClock),
                seq: at,
                at_ms: TimestampMs::from_millis(at),
                kind: TaskEventKind::Known(kind.clone()),
            }
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_task_events_1((kinds, split) in events_strategy()) {
        let events = materialize(&kinds);
        let full = replay(&events).unwrap();
        prop_assert_eq!(replay(&events).unwrap(), full.clone());
        let through = u64::try_from(split).unwrap_or(0);
        let snapshot = snapshot_at(&events, through);
        let joined = replay_from(&snapshot, &events).unwrap();
        prop_assert_eq!(joined, full);
    }
}

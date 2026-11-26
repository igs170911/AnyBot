use proptest::prelude::*;

use crate::fixtures_extra::context_built_kind;

use anybot_foundation::{CallId, EventId, SystemClock, TimestampMs};
use anybot_protocol::{
    BudgetItem, CheckSource, DoneWhenChange, KnownTaskEventKind, Proposer, TaskEvent,
    TaskEventKind, ToolEffect,
};

pub(crate) fn evidence_fixture(short: u32) -> anybot_protocol::EvidenceRecord {
    anybot_protocol::EvidenceRecord {
        id: anybot_foundation::EvidenceId::generate(&SystemClock),
        short: anybot_protocol::EvidenceShortId(short),
        at: TimestampMs::from_millis(0),
        task: anybot_foundation::TaskId::generate(&SystemClock),
        turn: anybot_foundation::TurnId::generate(&SystemClock),
        computer: "c".to_owned(),
        workdir: "/w".to_owned(),
        tool: format!("t{short}"),
        classes: Vec::new(),
        gate: anybot_protocol::GateRecord::new(
            anybot_protocol::GateNo(1),
            anybot_protocol::RuleLayer::Default,
            "r".to_owned(),
            anybot_protocol::JudgementSource::Rule,
            anybot_protocol::GateOutcome::Allow,
            false,
        ),
        kind: anybot_protocol::EvidenceKind::Seen,
        outcome: anybot_protocol::ActionOutcome::Completed,
        payload: anybot_protocol::EvidencePayload::Seen {
            path: format!("f{short}"),
            digest: anybot_foundation::ContentDigest::sha256(format!("d{short}").as_bytes()),
            size: 3,
        },
        verification: None,
        source: anybot_protocol::JudgementSource::Rule,
        ungated: false,
    }
}

pub(crate) fn ev(kind: KnownTaskEventKind) -> TaskEventKind {
    TaskEventKind::Known(kind)
}

pub(crate) fn known_event() -> BoxedStrategy<TaskEventKind> {
    use KnownTaskEventKind::{
        ActionVerified, BudgetUsage, CardShown, CriterionFailed, CriterionPassed, EvidenceAdded,
        ModelAnswer, ModelAsked, ModelSwapped, ModelUsage, OutcomeUnknown, ToolIntent, ToolNotRun,
    };
    prop_oneof![
        3 => crate::fixtures::criterion().prop_map(|c| ev(KnownTaskEventKind::DoneWhenChange {
            by: Proposer::User,
            change: DoneWhenChange::SetItems { items: vec![c] },
            before: Vec::new(),
            after: Vec::new(),
            reason: "r".to_owned(),
        })),
        10 => Just(ev(ModelAnswer {
            text: "t".to_owned(),
            thinking: None,
            tool_requests: Vec::new(),
            finish: anybot_protocol::FinishReason::Finished,
            native: None,
        })),
        6 => (0usize..3usize).prop_map(|i| ev(ToolIntent { call_id: CallId::generate(&SystemClock), name: format!("e{i}"), args_summary: "a".to_owned(), level: ToolEffect::ReadOnly })),
        4 => (0u32..9u32).prop_map(|i| {
            ev(EvidenceAdded {
                record: Box::new(evidence_fixture(i)),
            })
        }),
        2 => Just(ev(CardShown { action: "q?".to_owned() })),
        2 => Just(ev(OutcomeUnknown { call_id: CallId::generate(&SystemClock) })),
        1 => Just(ev(ActionVerified {
            call_id: CallId::generate(&SystemClock),
            source: CheckSource::Program,
            evidence_ids: Vec::new(),
        })),
        1 => Just(ev(BudgetUsage { item: BudgetItem::Tokens, delta: 5 })),
        1 => (0usize..3).prop_map(|i| ev(CriterionPassed { index: i, source: CheckSource::Program })),
        1 => (0usize..3).prop_map(|i| ev(CriterionFailed { index: i, source: CheckSource::Program, observed: "o".to_owned() })),
        1 => Just(ev(ModelSwapped { from: "a".to_owned(), to: "b".to_owned(), reason: "r".to_owned() })),
        1 => Just(ev(context_built_kind())),
        1 => Just(ev(ModelAsked { step: 1 })),
        1 => Just(ev(ModelUsage { usage: None, manifest_id: "m".to_owned() })),
        1 => (0usize..2).prop_map(|_| ev(ToolNotRun {
            call_id: CallId::generate(&SystemClock),
            reason: anybot_protocol::NotRunReason::NotWired,
        })),
    ]
    .boxed()
}

pub(crate) fn event_with_seq(seq: u64, kind: TaskEventKind) -> TaskEvent {
    TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq,
        at_ms: TimestampMs::from_millis(seq),
        kind,
    }
}

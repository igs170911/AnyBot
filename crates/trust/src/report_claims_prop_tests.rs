use anybot_foundation::{ContentDigest, EvidenceId, SystemClock, TaskId, TimestampMs, TurnId};
use anybot_protocol::{
    ActionOutcome, EvidenceKind, EvidencePayload, EvidenceRecord, EvidenceShortId, GateOutcome,
    GateRecord, GateStep, JudgementSource, ReportLevel, RuleLayer,
};
use proptest::prelude::*;

use crate::claim_levels::Scope;
use crate::report_claims::{VerifiedRef, grade_reply};

#[derive(Clone, Copy, Debug)]
enum CitationMode {
    Omitted,
    InTask,
    OtherTask,
}

#[derive(Clone, Copy, Debug)]
enum EvidenceShape {
    MatchingSend,
    WrongKindRead,
    RelayedBot,
}

#[derive(Clone, Copy, Debug)]
enum VerifyMode {
    None,
    Covering,
    WrongScope,
}

#[derive(Clone, Debug)]
struct CaseSpec {
    citation: CitationMode,
    completed: bool,
    shape: EvidenceShape,
    count_matches: bool,
    recipient_matches: bool,
    ungated: bool,
    verify: VerifyMode,
}

fn case_strategy() -> BoxedStrategy<CaseSpec> {
    (
        0usize..3usize,
        any::<bool>(),
        0usize..3usize,
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        0usize..3usize,
    )
        .prop_map(
            |(cit, completed, shp, count_matches, recipient_matches, ungated, ver)| CaseSpec {
                citation: match cit {
                    0 => CitationMode::Omitted,
                    1 => CitationMode::InTask,
                    _ => CitationMode::OtherTask,
                },
                completed,
                shape: match shp {
                    0 => EvidenceShape::MatchingSend,
                    1 => EvidenceShape::WrongKindRead,
                    _ => EvidenceShape::RelayedBot,
                },
                count_matches,
                recipient_matches,
                ungated,
                verify: match ver {
                    0 => VerifyMode::None,
                    1 => VerifyMode::Covering,
                    _ => VerifyMode::WrongScope,
                },
            },
        )
        .boxed()
}

fn build_payload(
    shape: EvidenceShape,
    recipient: &str,
) -> (String, EvidenceKind, EvidencePayload, Vec<String>) {
    match shape {
        EvidenceShape::RelayedBot => (
            "relay_bot".to_owned(),
            EvidenceKind::Approved,
            EvidencePayload::Relayed {
                from_bot: "Bot2".to_owned(),
                text_head: "done".to_owned(),
            },
            Vec::new(),
        ),
        EvidenceShape::MatchingSend => (
            "send_mail".to_owned(),
            EvidenceKind::Executed,
            EvidencePayload::Executed {
                command: format!("send_mail {recipient} out.csv"),
                computer: "bot-1".to_owned(),
                exit_code: Some(0),
                output_head: String::new(),
                output_tail: String::new(),
                output_omitted: 0,
                full_digest: ContentDigest::sha256(b"send"),
                elapsed_ms: 5,
            },
            vec![
                "send".to_owned(),
                format!("recipient:{recipient}"),
                "file:out.csv".to_owned(),
            ],
        ),
        EvidenceShape::WrongKindRead => (
            "read_file".to_owned(),
            EvidenceKind::Seen,
            EvidencePayload::Seen {
                path: "notes.txt".to_owned(),
                digest: ContentDigest::sha256(b"notes"),
                size: 5,
            },
            Vec::new(),
        ),
    }
}

fn build_record(spec: &CaseSpec, task: TaskId, other: TaskId) -> EvidenceRecord {
    let record_task = match spec.citation {
        CitationMode::OtherTask => other,
        CitationMode::Omitted | CitationMode::InTask => task,
    };
    let outcome = if spec.completed {
        ActionOutcome::Completed
    } else {
        ActionOutcome::Failed
    };
    let recipient = if spec.recipient_matches {
        "alice@example.com"
    } else {
        "bob@example.com"
    };
    let (tool, kind, payload, classes) = build_payload(spec.shape, recipient);
    EvidenceRecord {
        id: EvidenceId::generate(&SystemClock),
        short: EvidenceShortId(1),
        at: TimestampMs::from_millis(1000),
        task: record_task,
        turn: TurnId::generate(&SystemClock),
        computer: "bot-1".to_owned(),
        workdir: "/workspace".to_owned(),
        tool,
        classes,
        gate: GateRecord::new(
            GateStep::DefaultRule,
            RuleLayer::Default,
            "rule".to_owned(),
            JudgementSource::Rule,
            GateOutcome::Allow,
            false,
        ),
        kind,
        outcome,
        payload,
        verification: None,
        source: JudgementSource::Rule,
        ungated: spec.ungated,
    }
}

fn build_reply(spec: &CaseSpec) -> String {
    let cite = match spec.citation {
        CitationMode::Omitted => "",
        CitationMode::InTask | CitationMode::OtherTask => "\u{3014}E1\u{3015}",
    };
    if matches!(spec.shape, EvidenceShape::RelayedBot) {
        return format!("Bot2 \u{5df2}\u{5b8c}\u{6210}{cite}");
    }
    let count = if spec.count_matches { 1 } else { 3 };
    format!("\u{5df2}\u{9001}\u{51fa} {count} \u{5247} out.csv \u{7d66} alice@example.com{cite}")
}

fn expected_level(spec: &CaseSpec) -> ReportLevel {
    if !matches!(spec.citation, CitationMode::InTask) || !spec.completed {
        return ReportLevel::Unknown;
    }
    if matches!(spec.shape, EvidenceShape::RelayedBot) {
        return ReportLevel::Inferred;
    }
    if !matches!(spec.shape, EvidenceShape::MatchingSend)
        || !spec.count_matches
        || !spec.recipient_matches
    {
        return ReportLevel::Unknown;
    }
    if spec.ungated || !matches!(spec.verify, VerifyMode::Covering) {
        return ReportLevel::Sourced;
    }
    ReportLevel::Verified
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_report_delivery_11(spec in case_strategy()) {
        let task = TaskId::generate(&SystemClock);
        let other = TaskId::generate(&SystemClock);
        let record = build_record(&spec, task, other);
        let reply = build_reply(&spec);
        let verified = match spec.verify {
            VerifyMode::None => Vec::new(),
            VerifyMode::Covering => vec![VerifiedRef {
                evidence_id: Some(record.id),
                source: JudgementSource::ProgramCheck,
                scope: Some(Scope::BotComputer),
            }],
            VerifyMode::WrongScope => vec![VerifiedRef {
                evidence_id: Some(record.id),
                source: JudgementSource::ProgramCheck,
                scope: Some(Scope::UserComputer),
            }],
        };
        let first = grade_reply(&reply, &task, std::slice::from_ref(&record), &verified);
        let second = grade_reply(&reply, &task, std::slice::from_ref(&record), &verified);
        prop_assert_eq!(&first, &second);
        let claim = first.first().ok_or_else(|| TestCaseError::fail("missing claim"))?;
        let want = expected_level(&spec);
        prop_assert_eq!(claim.level, want);
        if want == ReportLevel::Unknown {
            prop_assert!(!claim.reasons.is_empty());
        } else {
            prop_assert!(claim.reasons.is_empty());
        }
    }
}

use anybot_foundation::{ContentDigest, EvidenceId, SystemClock, TaskId, TimestampMs, TurnId};
use anybot_protocol::{
    ActionOutcome, BeforeDigest, EvidenceKind, EvidencePayload, EvidenceRecord, EvidenceShortId,
    GateOutcome, GateRecord, GateStep, JudgementSource, ReportClaim, ReportLevel, RuleLayer,
};

use crate::claim_levels::Scope;
use crate::message_codes::{
    CODE_CLAIM_COUNT_MISMATCH, CODE_CLAIM_KIND_MISMATCH, CODE_CLAIM_NO_EVIDENCE,
    CODE_CLAIM_NOT_IN_TASK, CODE_CLAIM_RECIPIENT_MISMATCH, CODE_CLAIM_STEP_FAILED,
};
use crate::report_claims::{VerifiedRef, count_levels, grade_reply};

fn base_record(
    task: TaskId,
    short: u32,
    tool: &str,
    kind: EvidenceKind,
    outcome: ActionOutcome,
    payload: EvidencePayload,
) -> EvidenceRecord {
    EvidenceRecord {
        id: EvidenceId::generate(&SystemClock),
        short: EvidenceShortId(short),
        at: TimestampMs::from_millis(1000),
        task,
        turn: TurnId::generate(&SystemClock),
        computer: "bot-1".to_owned(),
        workdir: "/workspace".to_owned(),
        tool: tool.to_owned(),
        classes: Vec::new(),
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
        ungated: false,
    }
}

fn send_record(
    task: TaskId,
    short: u32,
    recipient: &str,
    outcome: ActionOutcome,
) -> EvidenceRecord {
    let mut rec = base_record(
        task,
        short,
        "send_mail",
        EvidenceKind::Executed,
        outcome,
        EvidencePayload::Executed {
            command: format!("send_mail {recipient} out.csv"),
            computer: "bot-1".to_owned(),
            exit_code: Some(0),
            output_head: String::new(),
            output_tail: String::new(),
            output_omitted: 0,
            full_digest: ContentDigest::sha256(b"ok"),
            elapsed_ms: 10,
        },
    );
    rec.classes = vec![
        "send".to_owned(),
        format!("recipient:{recipient}"),
        "file:out.csv".to_owned(),
    ];
    rec
}

fn build_fixture(task: TaskId, other: TaskId) -> (Vec<EvidenceRecord>, Vec<VerifiedRef>) {
    let e1 = send_record(task, 1, "bob@example.com", ActionOutcome::Completed);
    let e2 = base_record(
        task,
        2,
        "read_file",
        EvidenceKind::Seen,
        ActionOutcome::Completed,
        EvidencePayload::Seen {
            path: "a.txt".to_owned(),
            digest: ContentDigest::sha256(b"a"),
            size: 1,
        },
    );
    let e3_failed = send_record(task, 3, "alice@example.com", ActionOutcome::Failed);
    let e4_foreign = base_record(
        other,
        4,
        "edit_file",
        EvidenceKind::Changed,
        ActionOutcome::Completed,
        EvidencePayload::Changed {
            path: "b.txt".to_owned(),
            before: BeforeDigest::Absent,
            after: ContentDigest::sha256(b"b"),
            size: 1,
        },
    );
    let e5_relayed = base_record(
        task,
        5,
        "relay_bot",
        EvidenceKind::Approved,
        ActionOutcome::Completed,
        EvidencePayload::Relayed {
            from_bot: "Bot2".to_owned(),
            text_head: "ready".to_owned(),
        },
    );
    let mut e6_ungated = send_record(task, 6, "alice@example.com", ActionOutcome::Completed);
    e6_ungated.ungated = true;
    let verified = vec![
        VerifiedRef {
            evidence_id: Some(e1.id),
            source: JudgementSource::ProgramCheck,
            scope: Some(Scope::BotComputer),
        },
        VerifiedRef {
            evidence_id: Some(e6_ungated.id),
            source: JudgementSource::ProgramCheck,
            scope: Some(Scope::BotComputer),
        },
    ];
    (
        vec![e1, e2, e3_failed, e4_foreign, e5_relayed, e6_ungated],
        verified,
    )
}

fn first_claim(claims: Vec<ReportClaim>) -> ReportClaim {
    claims.into_iter().next().unwrap()
}

fn assert_unknown_reason(claim: &ReportClaim, code: &str) {
    assert_eq!(claim.level, ReportLevel::Unknown);
    assert!(claim.reasons.iter().any(|r| r.code == code));
}

#[test]
fn grade_reply_reason_codes() {
    let task = TaskId::generate(&SystemClock);
    let other = TaskId::generate(&SystemClock);
    let (evidence, verified) = build_fixture(task, other);

    let c_no_ev = first_claim(grade_reply(
        "\u{5df2}\u{5bc4}\u{51fa}",
        &task,
        &evidence,
        &verified,
    ));
    assert_unknown_reason(&c_no_ev, CODE_CLAIM_NO_EVIDENCE);

    let c_failed = first_claim(grade_reply(
        "\u{5df2}\u{5bc4}\u{51fa}\u{3014}E3\u{3015}",
        &task,
        &evidence,
        &verified,
    ));
    assert_unknown_reason(&c_failed, CODE_CLAIM_STEP_FAILED);

    let c_kind = first_claim(grade_reply(
        "\u{5df2}\u{5bc4}\u{7d66} alice\u{3014}E2\u{3015}",
        &task,
        &evidence,
        &verified,
    ));
    assert_unknown_reason(&c_kind, CODE_CLAIM_KIND_MISMATCH);
    assert_eq!(
        c_kind.reasons.first().unwrap().params,
        vec![("expected".to_owned(), "send".to_owned())]
    );

    let c_count_rcpt = first_claim(grade_reply(
        "\u{9001}\u{51fa} 3 \u{5247}\u{7d66} alice@example.com\u{3014}E1\u{3015}",
        &task,
        &evidence,
        &verified,
    ));
    assert_unknown_reason(&c_count_rcpt, CODE_CLAIM_COUNT_MISMATCH);
    assert_unknown_reason(&c_count_rcpt, CODE_CLAIM_RECIPIENT_MISMATCH);

    let c_foreign = first_claim(grade_reply(
        "\u{5df2}\u{6539}\u{597d} b.txt\u{3014}E4\u{3015}",
        &task,
        &evidence,
        &verified,
    ));
    assert_unknown_reason(&c_foreign, CODE_CLAIM_NOT_IN_TASK);

    check_positive_levels(&task, &evidence, &verified, c_no_ev);
}

fn check_positive_levels(
    task: &TaskId,
    evidence: &[EvidenceRecord],
    verified: &[VerifiedRef],
    unknown_claim: ReportClaim,
) {
    let c_relayed = first_claim(grade_reply(
        "Bot2 \u{5df2}\u{5b8c}\u{6210}\u{3014}E5\u{3015}",
        task,
        evidence,
        verified,
    ));
    assert_eq!(c_relayed.level, ReportLevel::Inferred);
    assert_eq!(c_relayed.source, Some(JudgementSource::ForwardedUnverified));

    let c_ungated = first_claim(grade_reply(
        "\u{5df2}\u{628a} out.csv \u{5bc4}\u{7d66} alice@example.com\u{3014}E6\u{3015}",
        task,
        evidence,
        verified,
    ));
    assert_eq!(c_ungated.level, ReportLevel::Sourced);

    let c_verified = first_claim(grade_reply(
        "\u{5df2}\u{628a} out.csv \u{5bc4}\u{7d66} bob@example.com\u{3014}E1\u{3015}",
        task,
        evidence,
        verified,
    ));
    assert_eq!(c_verified.level, ReportLevel::Verified);
    assert_eq!(c_verified.source, Some(JudgementSource::ProgramCheck));

    let counts = count_levels(&[unknown_claim, c_relayed, c_ungated, c_verified]);
    assert_eq!(counts.verified, 1);
    assert_eq!(counts.sourced, 1);
    assert_eq!(counts.inferred, 1);
    assert_eq!(counts.unknown, 1);
}

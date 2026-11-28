use anybot_foundation::{EvidenceId, TaskId};
use anybot_protocol::{
    EvidenceRecord, EvidenceShortId, JudgementSource, LevelCounts, ReasonPayload, ReportClaim,
    ReportLevel,
};

use crate::claim_levels::{
    Claim, EvidenceKind as LevelEvidenceKind, Link, Scope, Verification, claim_level,
    to_report_level,
};
use crate::message_codes::{CODE_CLAIM_NO_EVIDENCE, CODE_CLAIM_NOT_IN_TASK};
use crate::report_claim_checks::{check_records, is_relayed_record, push_reason};
use crate::report_sentences::{extract_citations, is_report_claim, split_report_sentences};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedRef {
    pub evidence_id: Option<EvidenceId>,
    pub source: JudgementSource,
    pub scope: Option<Scope>,
}

#[must_use]
pub fn grade_reply(
    reply: &str,
    task: &TaskId,
    evidence: &[EvidenceRecord],
    verified: &[VerifiedRef],
) -> Vec<ReportClaim> {
    grade_reply_inner(reply, Some(task), evidence, verified)
}

pub(crate) fn grade_reply_inner(
    reply: &str,
    task: Option<&TaskId>,
    evidence: &[EvidenceRecord],
    verified: &[VerifiedRef],
) -> Vec<ReportClaim> {
    let mut claims = Vec::new();
    for sentence in split_report_sentences(reply) {
        let cited = extract_citations(&sentence);
        if !is_report_claim(&sentence, &cited) {
            continue;
        }
        claims.push(grade_sentence(&sentence, &cited, task, evidence, verified));
    }
    claims
}

#[must_use]
pub fn count_levels(claims: &[ReportClaim]) -> LevelCounts {
    let mut counts = LevelCounts::default();
    for claim in claims {
        match claim.level {
            ReportLevel::Verified => counts.verified = counts.verified.saturating_add(1),
            ReportLevel::Sourced => counts.sourced = counts.sourced.saturating_add(1),
            ReportLevel::Inferred => counts.inferred = counts.inferred.saturating_add(1),
            ReportLevel::Unknown => counts.unknown = counts.unknown.saturating_add(1),
        }
    }
    counts
}

fn grade_sentence(
    sentence: &str,
    cited: &[EvidenceShortId],
    task: Option<&TaskId>,
    evidence: &[EvidenceRecord],
    verified: &[VerifiedRef],
) -> ReportClaim {
    if cited.is_empty() {
        return ReportClaim {
            sentence: sentence.to_owned(),
            level: ReportLevel::Unknown,
            reasons: vec![ReasonPayload {
                code: CODE_CLAIM_NO_EVIDENCE.to_owned(),
                params: Vec::new(),
            }],
            source: None,
            cited: Vec::new(),
        };
    }
    let mut reasons = Vec::new();
    let in_task = resolve_cited(cited, task, evidence, &mut reasons);
    if !in_task.is_empty() {
        check_records(sentence, &in_task, &mut reasons);
    }
    let (level, source) = if reasons.is_empty() {
        level_and_source(&in_task, verified)
    } else {
        (ReportLevel::Unknown, None)
    };
    ReportClaim {
        sentence: sentence.to_owned(),
        level,
        reasons,
        source,
        cited: cited.to_vec(),
    }
}

fn resolve_cited<'a>(
    cited: &[EvidenceShortId],
    task: Option<&TaskId>,
    evidence: &'a [EvidenceRecord],
    reasons: &mut Vec<ReasonPayload>,
) -> Vec<&'a EvidenceRecord> {
    let mut found = Vec::new();
    for short in cited {
        let record = evidence
            .iter()
            .find(|rec| rec.short == *short && task.is_some_and(|t| rec.task == *t));
        match record {
            Some(rec) => found.push(rec),
            None => push_reason(reasons, CODE_CLAIM_NOT_IN_TASK, Vec::new()),
        }
    }
    found
}

fn level_and_source(
    records: &[&EvidenceRecord],
    verified: &[VerifiedRef],
) -> (ReportLevel, Option<JudgementSource>) {
    let any_ungated = records.iter().any(|rec| rec.ungated);
    let all_relayed = records.iter().all(|rec| is_relayed_record(rec));
    let links: Vec<Link> = records
        .iter()
        .map(|rec| Link {
            kind: if is_relayed_record(rec) {
                LevelEvidenceKind::BotHearsay
            } else {
                LevelEvidenceKind::ToolReturn
            },
            gate_checked: !rec.ungated,
        })
        .collect();
    let claim_scope = records.first().map(|rec| scope_of_computer(&rec.computer));
    let covering = if all_relayed || any_ungated {
        None
    } else {
        covering_verification(records, verified, claim_scope)
    };
    let claim = Claim {
        text: String::new(),
        links,
        verification: covering
            .as_ref()
            .and_then(|v| v.scope.or(claim_scope).map(|scope| Verification { scope })),
        claim_scope,
    };
    let level = to_report_level(claim_level(&claim));
    let source = source_for_level(level, any_ungated, records, covering.as_ref());
    (level, source)
}

fn source_for_level(
    level: ReportLevel,
    any_ungated: bool,
    records: &[&EvidenceRecord],
    covering: Option<&VerifiedRef>,
) -> Option<JudgementSource> {
    match level {
        ReportLevel::Verified => covering.map(|v| v.source),
        ReportLevel::Sourced => Some(if any_ungated {
            JudgementSource::Ungated
        } else {
            records
                .first()
                .map_or(JudgementSource::Rule, |rec| rec.source)
        }),
        ReportLevel::Inferred => Some(JudgementSource::ForwardedUnverified),
        ReportLevel::Unknown => None,
    }
}

fn covering_verification(
    records: &[&EvidenceRecord],
    verified: &[VerifiedRef],
    claim_scope: Option<Scope>,
) -> Option<VerifiedRef> {
    let mut picked: Option<VerifiedRef> = None;
    for rec in records {
        let specific = verified
            .iter()
            .find(|v| v.evidence_id == Some(rec.id) && scope_matches(v.scope, claim_scope));
        let fallback = verified
            .iter()
            .find(|v| v.evidence_id.is_none() && scope_matches(v.scope, claim_scope));
        let found = specific.or(fallback)?;
        if picked.is_none() {
            picked = Some(found.clone());
        }
    }
    picked
}

fn scope_matches(verified_scope: Option<Scope>, claim_scope: Option<Scope>) -> bool {
    match (verified_scope, claim_scope) {
        (Some(v), Some(c)) => v == c,
        (None, _) => true,
        (Some(_), None) => false,
    }
}

fn scope_of_computer(computer: &str) -> Scope {
    let lower = computer.to_lowercase();
    if lower == "phone" {
        Scope::Phone
    } else if lower.contains("bot") {
        Scope::BotComputer
    } else {
        Scope::UserComputer
    }
}

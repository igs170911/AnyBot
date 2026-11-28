use anybot_protocol::{
    ActionOutcome, EvidenceKind, EvidencePayload, EvidenceRecord, ReasonPayload, VerificationResult,
};

use crate::claims_text::extract_claim_counts;
use crate::message_codes::{
    CODE_CLAIM_CHAT_DEAD_LINK, CODE_CLAIM_COUNT_MISMATCH, CODE_CLAIM_KIND_MISMATCH,
    CODE_CLAIM_NO_EVIDENCE, CODE_CLAIM_RECIPIENT_MISMATCH, CODE_CLAIM_STEP_FAILED,
};
use crate::report_extract::{extract_filenames, extract_recipients, extract_urls};
use crate::report_sentences::{ExpectedKind, expected_kind, strip_citations};
use crate::send_detection::{is_send_record, records_recipients};

pub(crate) fn check_records(
    sentence: &str,
    records: &[&EvidenceRecord],
    reasons: &mut Vec<ReasonPayload>,
) {
    if records.iter().any(|rec| step_failed(rec)) {
        push_reason(reasons, CODE_CLAIM_STEP_FAILED, Vec::new());
    }
    let exp = expected_kind(sentence);
    if let Some(kind) = exp
        && !records.iter().all(|rec| matches_expected_kind(rec, kind))
    {
        push_reason(
            reasons,
            CODE_CLAIM_KIND_MISMATCH,
            vec![("expected".to_owned(), kind.as_str().to_owned())],
        );
    }
    check_counts(sentence, records, exp, reasons);
    check_recipients(sentence, records, reasons);
    check_files_and_urls(sentence, records, reasons);
}

pub(crate) fn step_failed(rec: &EvidenceRecord) -> bool {
    rec.outcome != ActionOutcome::Completed
        || rec
            .verification
            .as_ref()
            .is_some_and(|v| v.result == VerificationResult::Failed)
}

fn matches_expected_kind(rec: &EvidenceRecord, expected: ExpectedKind) -> bool {
    match expected {
        ExpectedKind::Send => is_send_record(rec),
        ExpectedKind::Change => is_change_record(rec),
        ExpectedKind::Execute => is_execute_record(rec),
    }
}

pub(crate) fn is_change_record(rec: &EvidenceRecord) -> bool {
    matches!(rec.payload, EvidencePayload::Changed { .. }) || rec.kind == EvidenceKind::Changed
}

pub(crate) fn is_execute_record(rec: &EvidenceRecord) -> bool {
    matches!(rec.payload, EvidencePayload::Executed { .. }) && !is_send_record(rec)
}

pub(crate) fn is_relayed_record(rec: &EvidenceRecord) -> bool {
    matches!(rec.payload, EvidencePayload::Relayed { .. })
}

fn check_counts(
    sentence: &str,
    records: &[&EvidenceRecord],
    expected: Option<ExpectedKind>,
    reasons: &mut Vec<ReasonPayload>,
) {
    let clean = strip_citations(sentence);
    let (said_sends, said_files) = extract_claim_counts(&clean);
    if let Some(said) = said_sends {
        let found = records
            .iter()
            .filter(|rec| !step_failed(rec) && is_send_record(rec))
            .count();
        let found_u32 = u32::try_from(found).unwrap_or(u32::MAX);
        if said != found_u32 {
            push_count_mismatch(reasons, said, found_u32);
        }
    }
    if let Some(said) = said_files {
        let found = records
            .iter()
            .filter(|rec| !step_failed(rec) && matches_file_count_kind(rec, expected))
            .count();
        let found_u32 = u32::try_from(found).unwrap_or(u32::MAX);
        if said != found_u32 {
            push_count_mismatch(reasons, said, found_u32);
        }
    }
}

fn matches_file_count_kind(rec: &EvidenceRecord, expected: Option<ExpectedKind>) -> bool {
    match expected {
        Some(ExpectedKind::Send) => is_send_record(rec),
        _ => is_change_record(rec) || is_send_record(rec),
    }
}

fn push_count_mismatch(reasons: &mut Vec<ReasonPayload>, said: u32, found: u32) {
    push_reason(
        reasons,
        CODE_CLAIM_COUNT_MISMATCH,
        vec![
            ("said".to_owned(), said.to_string()),
            ("found".to_owned(), found.to_string()),
        ],
    );
}

fn check_recipients(sentence: &str, records: &[&EvidenceRecord], reasons: &mut Vec<ReasonPayload>) {
    let mentioned = extract_recipients(sentence);
    if mentioned.is_empty() {
        return;
    }
    let found = records_recipients(records);
    if found.is_empty() {
        return;
    }
    let mismatch = mentioned
        .iter()
        .any(|wanted| !found.iter().any(|got| recipient_matches(got, wanted)));
    if mismatch {
        push_reason(
            reasons,
            CODE_CLAIM_RECIPIENT_MISMATCH,
            vec![("found".to_owned(), found.join(", "))],
        );
    }
}

fn recipient_matches(found: &str, wanted: &str) -> bool {
    if found.eq_ignore_ascii_case(wanted) {
        return true;
    }
    if !wanted.contains('@')
        && let Some((local, _)) = found.split_once('@')
    {
        return local.eq_ignore_ascii_case(wanted);
    }
    false
}

fn check_files_and_urls(
    sentence: &str,
    records: &[&EvidenceRecord],
    reasons: &mut Vec<ReasonPayload>,
) {
    let record_files = records_file_targets(records);
    if record_files.is_empty() {
        return;
    }
    for url in extract_urls(sentence) {
        let url_base = basename_lower(&url);
        let matched = record_files
            .iter()
            .any(|target| target == &url.to_lowercase() || basename_lower(target) == url_base);
        if !matched {
            push_reason(reasons, CODE_CLAIM_CHAT_DEAD_LINK, Vec::new());
        }
    }
    for file in extract_filenames(sentence) {
        let matched = record_files
            .iter()
            .any(|target| basename_lower(target) == file);
        if !matched {
            push_reason(reasons, CODE_CLAIM_NO_EVIDENCE, Vec::new());
        }
    }
}

fn records_file_targets(records: &[&EvidenceRecord]) -> Vec<String> {
    let mut out = Vec::new();
    for rec in records {
        for class in &rec.classes {
            if let Some(val) = class.strip_prefix("file:") {
                push_unique_lower(&mut out, val);
            }
        }
        collect_payload_files(&rec.payload, &mut out);
    }
    out
}

fn collect_payload_files(payload: &EvidencePayload, out: &mut Vec<String>) {
    match payload {
        EvidencePayload::Changed { path, .. } | EvidencePayload::Seen { path, .. } => {
            push_unique_lower(out, path);
        }
        EvidencePayload::Executed { command, .. } => {
            for name in extract_filenames(command) {
                push_unique_lower(out, &name);
            }
            for url in extract_urls(command) {
                push_unique_lower(out, &url);
            }
        }
        EvidencePayload::Relayed { text_head, .. } => {
            for name in extract_filenames(text_head) {
                push_unique_lower(out, &name);
            }
        }
        EvidencePayload::Approved { .. }
        | EvidencePayload::Rejected { .. }
        | EvidencePayload::NotExecuted { .. } => {}
    }
}

fn push_unique_lower(out: &mut Vec<String>, item: &str) {
    let lower = item.trim().to_lowercase();
    if !lower.is_empty() && !out.contains(&lower) {
        out.push(lower);
    }
}

fn basename_lower(path: &str) -> String {
    let no_query = path.split(['?', '#']).next().unwrap_or(path);
    no_query
        .trim_end_matches(['/', '\\', '.', ',', ';'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_lowercase()
}

pub(crate) fn push_reason(
    reasons: &mut Vec<ReasonPayload>,
    code: &str,
    params: Vec<(String, String)>,
) {
    let candidate = ReasonPayload {
        code: code.to_owned(),
        params,
    };
    if !reasons.contains(&candidate) {
        reasons.push(candidate);
    }
}

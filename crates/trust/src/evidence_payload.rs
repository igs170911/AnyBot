use anybot_foundation::{ContentDigest, mask_secrets};
use anybot_protocol::{
    ActualResult, CommandOutcome, EvidenceKind, EvidencePayload, ListOutcome, PathOutcome,
    ReadOutcome, SearchOutcome, WriteOutcome,
};

const OUTPUT_EDGE_CHARS: usize = 6_000;

#[must_use]
pub fn payload_kind(result: &ActualResult) -> EvidenceKind {
    match result {
        ActualResult::Command(_) => EvidenceKind::Executed,
        ActualResult::Write(_) => EvidenceKind::Changed,
        ActualResult::Read(_)
        | ActualResult::List(_)
        | ActualResult::Search(_)
        | ActualResult::Path(_)
        | ActualResult::Caps(_) => EvidenceKind::Seen,
    }
}

#[must_use]
pub fn payload_of(result: &ActualResult) -> EvidencePayload {
    match result {
        ActualResult::Command(outcome) => command_payload(outcome),
        ActualResult::Write(outcome) => write_payload(outcome),
        ActualResult::Read(outcome) => read_payload(outcome),
        ActualResult::List(outcome) => list_payload(outcome),
        ActualResult::Search(outcome) => search_payload(outcome),
        ActualResult::Path(outcome) => path_payload(outcome),
        ActualResult::Caps(caps) => caps_payload(caps),
    }
}

#[must_use]
pub fn command_payload(outcome: &CommandOutcome) -> EvidencePayload {
    let (output_head, output_tail, output_omitted) = trim_evidence_output(&outcome.output);
    EvidencePayload::Executed {
        command: mask_secrets(&outcome.command),
        computer: outcome.computer.clone(),
        exit_code: outcome.exit_code,
        output_head,
        output_tail,
        output_omitted,
        full_digest: ContentDigest::sha256(outcome.output.as_bytes()),
        elapsed_ms: outcome.elapsed_ms,
    }
}

fn trim_evidence_output(output: &str) -> (String, String, u64) {
    let char_count = output.chars().count();
    let double_edge = OUTPUT_EDGE_CHARS.saturating_mul(2);
    if char_count <= double_edge {
        let masked = mask_secrets(output);
        return (masked.clone(), masked, 0);
    }
    let head_raw: String = output.chars().take(OUTPUT_EDGE_CHARS).collect();
    let tail_rev: Vec<char> = output.chars().rev().take(OUTPUT_EDGE_CHARS).collect();
    let tail_raw: String = tail_rev.into_iter().rev().collect();
    let omitted = u64::try_from(char_count.saturating_sub(double_edge)).unwrap_or(u64::MAX);
    (mask_secrets(&head_raw), mask_secrets(&tail_raw), omitted)
}

#[must_use]
pub fn write_payload(outcome: &WriteOutcome) -> EvidencePayload {
    EvidencePayload::Changed {
        path: outcome.path.clone(),
        before: outcome.before.clone(),
        after: outcome.after.clone(),
        size: outcome.size,
    }
}

#[must_use]
pub fn read_payload(outcome: &ReadOutcome) -> EvidencePayload {
    EvidencePayload::Seen {
        path: outcome.path.clone(),
        digest: outcome.digest.clone(),
        size: outcome.size,
    }
}

#[must_use]
pub fn caps_payload(caps: &anybot_protocol::ComputerCaps) -> EvidencePayload {
    let label = caps.capability_label();
    EvidencePayload::Seen {
        path: String::new(),
        digest: ContentDigest::sha256(label.as_bytes()),
        size: 0,
    }
}

#[must_use]
pub fn path_payload(outcome: &PathOutcome) -> EvidencePayload {
    EvidencePayload::Seen {
        path: outcome.canonical.clone(),
        digest: ContentDigest::sha256(outcome.canonical.as_bytes()),
        size: 0,
    }
}

#[must_use]
pub fn list_payload(outcome: &ListOutcome) -> EvidencePayload {
    EvidencePayload::Seen {
        path: outcome.canonical.clone(),
        digest: ContentDigest::sha256(outcome.canonical.as_bytes()),
        size: outcome.entries.len() as u64,
    }
}

#[must_use]
pub fn search_payload(outcome: &SearchOutcome) -> EvidencePayload {
    let count = if outcome.hits.is_empty() {
        outcome.files.len()
    } else {
        outcome.hits.len()
    };
    let summary: String = outcome
        .hits
        .iter()
        .map(|hit| format!("{}:{}:{}", hit.path, hit.line, hit.text))
        .chain(outcome.files.iter().cloned())
        .collect::<Vec<String>>()
        .join("\n");
    EvidencePayload::Seen {
        path: String::new(),
        digest: ContentDigest::sha256(summary.as_bytes()),
        size: count as u64,
    }
}

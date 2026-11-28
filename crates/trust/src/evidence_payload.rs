use anybot_foundation::ContentDigest;
use anybot_protocol::{
    ActualResult, CommandOutcome, EvidenceKind, EvidencePayload, ListOutcome, PathOutcome,
    ReadOutcome, SearchOutcome, WriteOutcome,
};

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
    EvidencePayload::Executed {
        command: outcome.command.clone(),
        computer: outcome.computer.clone(),
        exit_code: outcome.exit_code,
        output_head: outcome.output.clone(),
        output_tail: outcome.output.clone(),
        output_omitted: 0,
        full_digest: ContentDigest::sha256(outcome.output.as_bytes()),
        elapsed_ms: outcome.elapsed_ms,
    }
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

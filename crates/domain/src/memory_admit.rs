use anybot_foundation::RecordId;

use crate::memory_scan::{ScanFinding, scan};
use crate::memory_types::{
    MemoryBook, MemoryEventContent, MemoryEventKind, MemoryEvidenceRef, MemoryKind, MemoryOrigin,
    MemoryScope, MemorySource, MemoryState, ProjectedMemory, normalize_for_match,
    truncate_memory_content,
};

pub const CODE_MEMORY_EMPTY_CONTENT: &str = "memory-empty-content";
pub const CODE_MEMORY_CONTRADICTION_MISSING: &str = "memory-contradiction-target-missing";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryClaimSource {
    UserMessage {
        record_id: RecordId,
        message_text: String,
    },
    OwnerDirect,
    ToolEvidence {
        evidence_refs: Vec<MemoryEvidenceRef>,
    },
    ModelInference,
    OtherBot {
        evidence_refs: Vec<MemoryEvidenceRef>,
    },
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryProposal {
    pub memory_id: String,
    pub scope: MemoryScope,
    pub kind: MemoryKind,
    pub origin: MemoryOrigin,
    pub raw_content: String,
    pub claim: MemoryClaimSource,
    pub evidence_refs: Vec<MemoryEvidenceRef>,
    pub tags: Vec<String>,
    pub valid_from_ms: Option<i64>,
    pub valid_to_ms: Option<i64>,
    pub contradicts: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedMemory {
    pub event_kind: MemoryEventKind,
    pub event_content: MemoryEventContent,
    pub initial_state: MemoryState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmitReject {
    ScanBlocked {
        code: &'static str,
        findings: Vec<ScanFinding>,
    },
    EmptyContent {
        code: &'static str,
    },
    ContradictionTargetNotFound {
        code: &'static str,
        target_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admission {
    Admit(Box<AdmittedMemory>),
    Duplicate { existing_id: String },
    Restore { memory_id: String },
    Rejected(AdmitReject),
}

#[must_use]
pub fn admit(proposal: &MemoryProposal, book: &MemoryBook) -> Admission {
    let findings = scan(&proposal.raw_content);
    if let Some(first) = findings.first() {
        return Admission::Rejected(AdmitReject::ScanBlocked {
            code: first.category.error_code(),
            findings,
        });
    }
    let trimmed = proposal.raw_content.trim();
    if trimmed.is_empty() {
        return Admission::Rejected(AdmitReject::EmptyContent {
            code: CODE_MEMORY_EMPTY_CONTENT,
        });
    }
    let content = truncate_memory_content(trimmed);
    let normalized = normalize_for_match(&content);
    if let Some(retracted) = book.find_retracted_normalized(proposal.scope, &normalized) {
        return Admission::Restore {
            memory_id: retracted.memory_id.clone(),
        };
    }
    if let Some(existing) = book.find_active_normalized(proposal.scope, &normalized) {
        return Admission::Duplicate {
            existing_id: existing.memory_id.clone(),
        };
    }
    if let Some(target_id) = &proposal.contradicts
        && !book.get(target_id).is_some_and(ProjectedMemory::is_active)
    {
        return Admission::Rejected(AdmitReject::ContradictionTargetNotFound {
            code: CODE_MEMORY_CONTRADICTION_MISSING,
            target_id: target_id.clone(),
        });
    }
    let (source, evidence_refs, initial_state) = resolve_source_and_state(proposal, &normalized);
    let final_state = if proposal.contradicts.is_some() {
        MemoryState::Contradicted
    } else {
        initial_state
    };
    Admission::Admit(Box::new(AdmittedMemory {
        event_kind: MemoryEventKind::Added,
        event_content: build_added_content(proposal, content, source, evidence_refs),
        initial_state: final_state,
    }))
}

fn resolve_source_and_state(
    proposal: &MemoryProposal,
    normalized_content: &str,
) -> (MemorySource, Vec<MemoryEvidenceRef>, MemoryState) {
    let mut refs = proposal.evidence_refs.clone();
    match &proposal.claim {
        MemoryClaimSource::UserMessage {
            record_id,
            message_text,
        } => {
            let normalized_msg = normalize_for_match(message_text);
            if !normalized_content.is_empty() && normalized_msg.contains(normalized_content) {
                let user_ref = MemoryEvidenceRef::UserRecord {
                    record_id: *record_id,
                };
                if !refs.contains(&user_ref) {
                    refs.insert(0, user_ref);
                }
                (MemorySource::UserSaid, refs, MemoryState::Verified)
            } else {
                (MemorySource::ModelInference, refs, MemoryState::Candidate)
            }
        }
        MemoryClaimSource::OwnerDirect => (MemorySource::UserSaid, refs, MemoryState::Verified),
        MemoryClaimSource::ToolEvidence { evidence_refs } => {
            append_refs(&mut refs, evidence_refs);
            if refs.is_empty() {
                (MemorySource::ModelInference, refs, MemoryState::Candidate)
            } else {
                (MemorySource::ToolEvidence, refs, MemoryState::Supported)
            }
        }
        MemoryClaimSource::ModelInference => {
            (MemorySource::ModelInference, refs, MemoryState::Candidate)
        }
        MemoryClaimSource::OtherBot { evidence_refs } => {
            append_refs(&mut refs, evidence_refs);
            (MemorySource::OtherBot, refs, MemoryState::Candidate)
        }
        MemoryClaimSource::Import => (MemorySource::Import, refs, MemoryState::Candidate),
    }
}

fn append_refs(target: &mut Vec<MemoryEvidenceRef>, incoming: &[MemoryEvidenceRef]) {
    for item in incoming {
        if !target.contains(item) {
            target.push(item.clone());
        }
    }
}

fn build_added_content(
    proposal: &MemoryProposal,
    content: String,
    source: MemorySource,
    evidence_refs: Vec<MemoryEvidenceRef>,
) -> MemoryEventContent {
    MemoryEventContent {
        memory_id: proposal.memory_id.clone(),
        version: 1,
        prev_version: None,
        kind: Some(proposal.kind),
        origin: Some(proposal.origin),
        content: Some(content),
        source: Some(source),
        evidence_refs,
        scope: Some(proposal.scope),
        tags: proposal.tags.clone(),
        valid_from_ms: proposal.valid_from_ms,
        valid_to_ms: proposal.valid_to_ms,
        replaced_by: None,
        contradicts: proposal.contradicts.clone(),
        reason: proposal.reason.clone(),
    }
}

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

pub use anybot_protocol::{
    MemoryEventContent, MemoryEventKind, MemoryEvidenceRef, MemoryKind, MemoryOrigin, MemoryScope,
    MemorySource, StoredRecord,
};

pub const MAX_MEMORY_CHARS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryState {
    Candidate,
    Supported,
    Verified,
    Outdated,
    Contradicted,
    Archived,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEventRecord {
    pub seq: i64,
    pub created_at_ms: i64,
    pub event_kind: MemoryEventKind,
    pub content: MemoryEventContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecordError {
    pub seq: i64,
    pub reason: String,
}

impl MemoryEventRecord {
    pub fn from_stored(stored: &StoredRecord) -> Result<Option<Self>, MemoryRecordError> {
        let Some(event_kind) = MemoryEventKind::from_kind_name(&stored.kind) else {
            return Ok(None);
        };
        let content: MemoryEventContent =
            serde_json::from_value(stored.content.clone()).map_err(|err| MemoryRecordError {
                seq: stored.seq,
                reason: err.to_string(),
            })?;
        Ok(Some(Self {
            seq: stored.seq,
            created_at_ms: stored.created_at_ms,
            event_kind,
            content,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContradictionNote {
    pub other_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryVersionEntry {
    pub version: u64,
    pub seq: i64,
    pub at_ms: i64,
    pub content: String,
    pub kind: MemoryKind,
    pub origin: MemoryOrigin,
    pub source: MemorySource,
    pub scope: MemoryScope,
    pub tags: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedMemory {
    pub memory_id: String,
    pub version: u64,
    pub scope: MemoryScope,
    pub kind: MemoryKind,
    pub origin: MemoryOrigin,
    pub state: MemoryState,
    pub source: MemorySource,
    pub content: String,
    pub evidence_refs: Vec<MemoryEvidenceRef>,
    pub tags: Vec<String>,
    pub valid_from_ms: Option<i64>,
    pub valid_to_ms: Option<i64>,
    pub replaced_by: Option<String>,
    pub pinned: bool,
    pub retracted_at_ms: Option<i64>,
    pub state_before_retract: Option<MemoryState>,
    pub contradictions: Vec<ContradictionNote>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub last_seq: i64,
    pub versions: Vec<MemoryVersionEntry>,
}

impl ProjectedMemory {
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.retracted_at_ms.is_none()
            && !matches!(
                self.state,
                MemoryState::Outdated | MemoryState::Archived | MemoryState::Superseded
            )
    }

    #[must_use]
    pub fn is_candidate(&self) -> bool {
        self.state == MemoryState::Candidate
    }

    #[must_use]
    pub fn state_at(&self, now_ms: i64) -> MemoryState {
        if self.is_active() && self.valid_to_ms.is_some_and(|end| end <= now_ms) {
            return MemoryState::Outdated;
        }
        self.state
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBook {
    pub entries: BTreeMap<String, ProjectedMemory>,
    pub last_seq: i64,
}

#[must_use]
pub fn truncate_memory_content(raw: &str) -> String {
    raw.chars().take(MAX_MEMORY_CHARS).collect()
}

#[must_use]
pub fn normalize_for_match(text: &str) -> String {
    let nfkc: String = text.nfkc().collect();
    let lower = nfkc.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut prev_space = true;
    for ch in lower.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

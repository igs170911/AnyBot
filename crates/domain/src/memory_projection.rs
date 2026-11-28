use std::collections::{BTreeMap, BTreeSet};

use anybot_foundation::BotId;

use crate::memory_projection_ops::{
    apply_confirmed, apply_pinned, apply_restored, apply_retracted, apply_superseded, apply_voided,
};
use crate::memory_types::{
    ContradictionNote, MemoryBook, MemoryEventContent, MemoryEventKind, MemoryEventRecord,
    MemoryEvidenceRef, MemoryKind, MemoryOrigin, MemoryScope, MemorySource, MemoryState,
    MemoryVersionEntry, ProjectedMemory, normalize_for_match, truncate_memory_content,
};

type PendingLinks = BTreeMap<String, Vec<(String, Option<String>)>>;

#[must_use]
pub fn project(events: &[MemoryEventRecord]) -> MemoryBook {
    let mut book = MemoryBook::default();
    let mut pending: PendingLinks = BTreeMap::new();
    for event in events {
        apply_event(&mut book, &mut pending, event);
        book.last_seq = book.last_seq.max(event.seq);
    }
    book
}

impl MemoryBook {
    #[must_use]
    pub fn get(&self, memory_id: &str) -> Option<&ProjectedMemory> {
        self.entries.get(memory_id)
    }

    #[must_use]
    pub fn resolve_redirect<'a>(&'a self, memory_id: &'a str) -> Option<&'a ProjectedMemory> {
        let mut cursor = memory_id;
        let mut visited = BTreeSet::new();
        while visited.insert(cursor) {
            let entry = self.entries.get(cursor)?;
            match &entry.replaced_by {
                Some(next) if entry.state == MemoryState::Superseded => {
                    cursor = next.as_str();
                }
                Some(_) | None => return Some(entry),
            }
        }
        None
    }

    #[must_use]
    pub fn active_entries(&self) -> Vec<&ProjectedMemory> {
        self.entries
            .values()
            .filter(|entry| entry.is_active())
            .collect()
    }

    #[must_use]
    pub fn about_you_snapshot(&self, limit: usize) -> Vec<&ProjectedMemory> {
        let mut items: Vec<&ProjectedMemory> = self
            .entries
            .values()
            .filter(|entry| entry.is_active() && entry.scope == MemoryScope::AboutYou)
            .collect();
        items.sort_by(|a, b| {
            b.created_at_ms
                .cmp(&a.created_at_ms)
                .then_with(|| b.last_seq.cmp(&a.last_seq))
                .then_with(|| b.memory_id.cmp(&a.memory_id))
        });
        items.truncate(limit);
        items
    }

    #[must_use]
    pub fn pinned_for_bot(&self, bot_id: BotId) -> Vec<&ProjectedMemory> {
        self.entries
            .values()
            .filter(|entry| {
                entry.is_active() && entry.pinned && entry.scope == MemoryScope::Bot(bot_id)
            })
            .collect()
    }

    #[must_use]
    pub fn find_active_normalized(
        &self,
        scope: MemoryScope,
        normalized: &str,
    ) -> Option<&ProjectedMemory> {
        self.entries.values().find(|entry| {
            entry.is_active()
                && entry.scope == scope
                && normalize_for_match(&entry.content) == normalized
        })
    }

    #[must_use]
    pub fn find_retracted_normalized(
        &self,
        scope: MemoryScope,
        normalized: &str,
    ) -> Option<&ProjectedMemory> {
        self.entries.values().find(|entry| {
            entry.retracted_at_ms.is_some()
                && !matches!(entry.state, MemoryState::Superseded | MemoryState::Outdated)
                && entry.scope == scope
                && normalize_for_match(&entry.content) == normalized
        })
    }
}

fn apply_event(book: &mut MemoryBook, pending: &mut PendingLinks, event: &MemoryEventRecord) {
    match event.event_kind {
        MemoryEventKind::Added => apply_added(book, pending, event),
        MemoryEventKind::Superseded => apply_superseded(book, event),
        MemoryEventKind::Retracted => apply_retracted(book, event),
        MemoryEventKind::Confirmed => apply_confirmed(book, event),
        MemoryEventKind::Pinned => apply_pinned(book, event, true),
        MemoryEventKind::Unpinned => apply_pinned(book, event, false),
        MemoryEventKind::Voided => apply_voided(book, event),
        MemoryEventKind::Restored => apply_restored(book, event),
    }
}

fn apply_added(book: &mut MemoryBook, pending: &mut PendingLinks, event: &MemoryEventRecord) {
    let payload = &event.content;
    if book.entries.contains_key(&payload.memory_id) {
        return;
    }
    let Some(raw) = &payload.content else {
        return;
    };
    let Some(scope) = payload.scope else {
        return;
    };
    let entry = build_new_entry(event, payload, raw, scope);
    book.entries.insert(payload.memory_id.clone(), entry);
    if let Some(other_id) = &payload.contradicts {
        link_contradiction(
            book,
            pending,
            &payload.memory_id,
            other_id,
            payload.reason.clone(),
        );
    }
    resolve_pending_for(book, pending, &payload.memory_id);
}

fn build_new_entry(
    event: &MemoryEventRecord,
    payload: &MemoryEventContent,
    raw: &str,
    scope: MemoryScope,
) -> ProjectedMemory {
    let content = truncate_memory_content(raw);
    let source = payload.source.unwrap_or(MemorySource::ModelInference);
    let kind = payload.kind.unwrap_or(MemoryKind::Knowledge);
    let origin = payload.origin.unwrap_or(MemoryOrigin::Remembered);
    let state = initial_state(source, &payload.evidence_refs);
    let version = payload.version.max(1);
    let history = MemoryVersionEntry {
        version,
        seq: event.seq,
        at_ms: event.created_at_ms,
        content: content.clone(),
        kind,
        origin,
        source,
        scope,
        tags: payload.tags.clone(),
        reason: payload.reason.clone(),
    };
    ProjectedMemory {
        memory_id: payload.memory_id.clone(),
        version,
        scope,
        kind,
        origin,
        state,
        source,
        content,
        evidence_refs: payload.evidence_refs.clone(),
        tags: payload.tags.clone(),
        valid_from_ms: payload.valid_from_ms,
        valid_to_ms: payload.valid_to_ms,
        replaced_by: None,
        pinned: false,
        retracted_at_ms: None,
        state_before_retract: None,
        contradictions: Vec::new(),
        created_at_ms: event.created_at_ms,
        updated_at_ms: event.created_at_ms,
        last_seq: event.seq,
        versions: vec![history],
    }
}

fn initial_state(source: MemorySource, refs: &[MemoryEvidenceRef]) -> MemoryState {
    match source {
        MemorySource::UserSaid => MemoryState::Verified,
        MemorySource::ToolEvidence if !refs.is_empty() => MemoryState::Supported,
        MemorySource::ToolEvidence
        | MemorySource::ModelInference
        | MemorySource::OtherBot
        | MemorySource::Import => MemoryState::Candidate,
    }
}

fn link_contradiction(
    book: &mut MemoryBook,
    pending: &mut PendingLinks,
    new_id: &str,
    other_id: &str,
    reason: Option<String>,
) {
    if new_id == other_id {
        return;
    }
    let other_active = book
        .entries
        .get(other_id)
        .is_some_and(|e| !matches!(e.state, MemoryState::Superseded | MemoryState::Outdated));
    if !other_active {
        if !book.entries.contains_key(other_id) {
            pending
                .entry(other_id.to_owned())
                .or_default()
                .push((new_id.to_owned(), reason));
        }
        return;
    }
    add_contradiction_note(book, new_id, other_id, reason.clone());
    add_contradiction_note(book, other_id, new_id, reason);
}

fn resolve_pending_for(book: &mut MemoryBook, pending: &mut PendingLinks, arrived_id: &str) {
    let Some(waiters) = pending.remove(arrived_id) else {
        return;
    };
    for (other_id, reason) in waiters {
        let other_eligible = book
            .entries
            .get(&other_id)
            .is_some_and(|e| !matches!(e.state, MemoryState::Superseded | MemoryState::Outdated));
        if other_eligible {
            add_contradiction_note(book, arrived_id, &other_id, reason.clone());
            add_contradiction_note(book, &other_id, arrived_id, reason);
        }
    }
}

fn add_contradiction_note(book: &mut MemoryBook, id: &str, other_id: &str, reason: Option<String>) {
    if let Some(entry) = book.entries.get_mut(id) {
        entry.state = MemoryState::Contradicted;
        if entry.retracted_at_ms.is_some() {
            entry.state_before_retract = Some(MemoryState::Contradicted);
        }
        if !entry.contradictions.iter().any(|c| c.other_id == other_id) {
            entry.contradictions.push(ContradictionNote {
                other_id: other_id.to_owned(),
                reason,
            });
        }
    }
}

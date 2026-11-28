use crate::memory_types::{
    MemoryBook, MemoryEventContent, MemoryEventRecord, MemoryEvidenceRef, MemoryScope,
    MemorySource, MemoryState, MemoryVersionEntry, ProjectedMemory, truncate_memory_content,
};

pub(crate) fn apply_confirmed(book: &mut MemoryBook, event: &MemoryEventRecord) {
    let payload = &event.content;
    let Some(entry) = book.entries.get(&payload.memory_id) else {
        return;
    };
    if matches!(entry.state, MemoryState::Superseded | MemoryState::Outdated) {
        return;
    }
    match payload.source {
        None | Some(MemorySource::UserSaid) => confirm_by_user(book, event),
        Some(MemorySource::ToolEvidence) if !payload.evidence_refs.is_empty() => {
            confirm_by_tool(book, event);
        }
        Some(
            MemorySource::ToolEvidence
            | MemorySource::ModelInference
            | MemorySource::OtherBot
            | MemorySource::Import,
        ) => {}
    }
}

fn confirm_by_user(book: &mut MemoryBook, event: &MemoryEventRecord) {
    let id = &event.content.memory_id;
    let other_ids: Vec<String> = {
        let Some(entry) = book.entries.get_mut(id) else {
            return;
        };
        entry.state = MemoryState::Verified;
        if entry.retracted_at_ms.is_some() {
            entry.state_before_retract = Some(MemoryState::Verified);
        }
        merge_evidence(&mut entry.evidence_refs, &event.content.evidence_refs);
        entry.updated_at_ms = event.created_at_ms;
        entry.last_seq = event.seq;
        entry
            .contradictions
            .iter()
            .map(|c| c.other_id.clone())
            .collect()
    };
    for other_id in other_ids {
        if let Some(other) = book.entries.get_mut(&other_id)
            && other.state == MemoryState::Contradicted
        {
            other.state = MemoryState::Outdated;
            other.pinned = false;
            other.valid_to_ms = other.valid_to_ms.or(Some(event.created_at_ms));
            other.updated_at_ms = event.created_at_ms;
            other.last_seq = event.seq;
        }
    }
}

fn confirm_by_tool(book: &mut MemoryBook, event: &MemoryEventRecord) {
    let Some(entry) = book.entries.get_mut(&event.content.memory_id) else {
        return;
    };
    merge_evidence(&mut entry.evidence_refs, &event.content.evidence_refs);
    if entry.state == MemoryState::Candidate {
        entry.state = MemoryState::Supported;
    }
    if entry.state_before_retract == Some(MemoryState::Candidate) {
        entry.state_before_retract = Some(MemoryState::Supported);
    }
    entry.updated_at_ms = event.created_at_ms;
    entry.last_seq = event.seq;
}

pub(crate) fn apply_superseded(book: &mut MemoryBook, event: &MemoryEventRecord) {
    let payload = &event.content;
    let Some(entry) = book.entries.get_mut(&payload.memory_id) else {
        return;
    };
    if matches!(entry.state, MemoryState::Superseded | MemoryState::Outdated) {
        return;
    }
    if let Some(target) = &payload.replaced_by {
        entry.state = MemoryState::Superseded;
        entry.replaced_by = Some(target.clone());
        entry.pinned = false;
        entry.updated_at_ms = event.created_at_ms;
        entry.last_seq = event.seq;
        return;
    }
    apply_in_place_edit(entry, event);
}

fn apply_in_place_edit(entry: &mut ProjectedMemory, event: &MemoryEventRecord) {
    let payload = &event.content;
    let from_model = matches!(
        payload.source,
        None | Some(MemorySource::ModelInference | MemorySource::OtherBot | MemorySource::Import)
    );
    if from_model && entry.state != MemoryState::Candidate {
        return;
    }
    if let Some(raw) = &payload.content {
        entry.content = truncate_memory_content(raw);
    }
    if let Some(kind) = payload.kind {
        entry.kind = kind;
    }
    if let Some(origin) = payload.origin {
        entry.origin = origin;
    }
    if !payload.tags.is_empty() {
        entry.tags.clone_from(&payload.tags);
    }
    if payload.valid_from_ms.is_some() {
        entry.valid_from_ms = payload.valid_from_ms;
    }
    if payload.valid_to_ms.is_some() {
        entry.valid_to_ms = payload.valid_to_ms;
    }
    merge_evidence(&mut entry.evidence_refs, &payload.evidence_refs);
    update_state_on_edit(entry, payload);
    entry.version = payload.version.max(entry.version.saturating_add(1));
    entry.updated_at_ms = event.created_at_ms;
    entry.last_seq = event.seq;
    entry.versions.push(MemoryVersionEntry {
        version: entry.version,
        seq: event.seq,
        at_ms: event.created_at_ms,
        content: entry.content.clone(),
        kind: entry.kind,
        origin: entry.origin,
        source: payload.source.unwrap_or(entry.source),
        scope: entry.scope,
        tags: entry.tags.clone(),
        reason: payload.reason.clone(),
    });
}

fn update_state_on_edit(entry: &mut ProjectedMemory, payload: &MemoryEventContent) {
    match payload.source {
        Some(MemorySource::UserSaid) => entry.state = MemoryState::Verified,
        Some(MemorySource::ToolEvidence)
            if !payload.evidence_refs.is_empty() && entry.state == MemoryState::Candidate =>
        {
            entry.state = MemoryState::Supported;
        }
        None
        | Some(
            MemorySource::ToolEvidence
            | MemorySource::ModelInference
            | MemorySource::OtherBot
            | MemorySource::Import,
        ) => {}
    }
}

pub(crate) fn apply_retracted(book: &mut MemoryBook, event: &MemoryEventRecord) {
    let Some(entry) = book.entries.get_mut(&event.content.memory_id) else {
        return;
    };
    if matches!(entry.state, MemoryState::Superseded | MemoryState::Outdated) {
        return;
    }
    if entry.retracted_at_ms.is_none() {
        entry.state_before_retract = Some(entry.state);
        entry.retracted_at_ms = Some(event.created_at_ms);
        entry.updated_at_ms = event.created_at_ms;
        entry.last_seq = event.seq;
    }
}

pub(crate) fn apply_restored(book: &mut MemoryBook, event: &MemoryEventRecord) {
    let Some(entry) = book.entries.get_mut(&event.content.memory_id) else {
        return;
    };
    if matches!(entry.state, MemoryState::Superseded | MemoryState::Outdated) {
        return;
    }
    if entry.retracted_at_ms.is_some() {
        entry.retracted_at_ms = None;
        if let Some(prev) = entry.state_before_retract.take() {
            entry.state = prev;
        }
        entry.updated_at_ms = event.created_at_ms;
        entry.last_seq = event.seq;
    }
}

pub(crate) fn apply_pinned(book: &mut MemoryBook, event: &MemoryEventRecord, pinned: bool) {
    let Some(entry) = book.entries.get_mut(&event.content.memory_id) else {
        return;
    };
    if matches!(entry.state, MemoryState::Superseded | MemoryState::Outdated) {
        return;
    }
    if pinned && entry.scope == MemoryScope::AboutYou {
        return;
    }
    entry.pinned = pinned;
    entry.updated_at_ms = event.created_at_ms;
    entry.last_seq = event.seq;
}

pub(crate) fn apply_voided(book: &mut MemoryBook, event: &MemoryEventRecord) {
    let Some(entry) = book.entries.get_mut(&event.content.memory_id) else {
        return;
    };
    if entry.state == MemoryState::Superseded {
        return;
    }
    entry.state = MemoryState::Outdated;
    entry.pinned = false;
    entry.valid_to_ms = entry.valid_to_ms.or(Some(event.created_at_ms));
    entry.updated_at_ms = event.created_at_ms;
    entry.last_seq = event.seq;
}

fn merge_evidence(target: &mut Vec<MemoryEvidenceRef>, incoming: &[MemoryEvidenceRef]) {
    for item in incoming {
        if !target.contains(item) {
            target.push(item.clone());
        }
    }
}

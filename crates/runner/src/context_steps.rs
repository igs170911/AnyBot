use std::collections::BTreeMap;

use anybot_context::StepOutcomeKind;
use anybot_foundation::{CallId, TaskId};
use anybot_protocol::{GateNo, KnownTaskEventKind, ReasonPayload};

use crate::context_denied::{CallResult, denied_text, not_run_word};
use crate::context_input::{known_of, row_event_id};
use crate::error::TurnError;
use crate::record::Stored;

pub(crate) fn results_of(
    stored: &[Stored],
    task: &TaskId,
    lang: anybot_messages::Lang,
) -> BTreeMap<CallId, CallResult> {
    let mut map: BTreeMap<CallId, CallResult> = BTreeMap::new();
    let mut pool: BTreeMap<anybot_foundation::EvidenceId, anybot_protocol::EvidenceRecord> =
        BTreeMap::new();
    let mut gates: Vec<(GateNo, ReasonPayload)> = Vec::new();
    let mut rows: Vec<(&Stored, KnownTaskEventKind, anybot_foundation::EventId)> = Vec::new();
    for row in stored {
        let Some(kind) = known_of(row, task) else {
            continue;
        };
        let Some(event_id) = row_event_id(row) else {
            continue;
        };
        crate::context_denied::collect_gate(&mut gates, kind);
        rows.push((row, kind.clone(), event_id));
    }
    for (row, kind, event_id) in rows {
        apply_row(
            &mut map,
            &mut pool,
            &gates,
            &kind,
            row.stream_seq,
            event_id,
            lang,
        );
    }
    map
}

fn apply_row(
    map: &mut BTreeMap<CallId, CallResult>,
    pool: &mut BTreeMap<anybot_foundation::EvidenceId, anybot_protocol::EvidenceRecord>,
    gates: &[(GateNo, ReasonPayload)],
    kind: &KnownTaskEventKind,
    seq: u64,
    event_id: anybot_foundation::EventId,
    lang: anybot_messages::Lang,
) {
    if let Ok(Some(entry)) = result_entry(kind, seq, event_id, lang, gates) {
        map.insert(entry.0, entry.1);
        return;
    }
    if let KnownTaskEventKind::EvidenceAdded { record } = kind {
        pool.insert(record.id, (**record).clone());
        return;
    }
    if let KnownTaskEventKind::ActionVerified {
        call_id,
        evidence_ids,
        ..
    } = kind
    {
        attach_verified(map, pool, *call_id, evidence_ids);
    }
}

fn result_entry(
    kind: &KnownTaskEventKind,
    seq: u64,
    event_id: anybot_foundation::EventId,
    lang: anybot_messages::Lang,
    gates: &[(GateNo, ReasonPayload)],
) -> Result<Option<(CallId, CallResult)>, TurnError> {
    if let KnownTaskEventKind::GateResult { call_id, gate } = kind {
        if gate.outcome != anybot_protocol::GateOutcome::Deny {
            return Ok(None);
        }
        let text = denied_text(lang, gate, gates)?;
        let found = CallResult::of(seq, event_id, StepOutcomeKind::Completed, text);
        return Ok(Some((*call_id, found)));
    }
    if let KnownTaskEventKind::ToolCompleted {
        call_id, summary, ..
    } = kind
    {
        let found = CallResult::of(seq, event_id, StepOutcomeKind::Completed, summary.clone());
        return Ok(Some((*call_id, found)));
    }
    if let KnownTaskEventKind::ToolFailed { call_id, error, .. } = kind {
        let found = CallResult::of(seq, event_id, StepOutcomeKind::Failed, error.clone());
        return Ok(Some((*call_id, found)));
    }
    if let KnownTaskEventKind::ToolNotRun { call_id, reason } = kind {
        let found = CallResult::of(
            seq,
            event_id,
            StepOutcomeKind::Unknown,
            not_run_word(*reason),
        );
        return Ok(Some((*call_id, found)));
    }
    Ok(None)
}

fn attach_verified(
    map: &mut BTreeMap<CallId, CallResult>,
    pool: &BTreeMap<anybot_foundation::EvidenceId, anybot_protocol::EvidenceRecord>,
    call_id: CallId,
    evidence_ids: &[anybot_foundation::EvidenceId],
) {
    if let Some(entry) = map.get_mut(&call_id) {
        for id in evidence_ids {
            if let Some(record) = pool.get(id) {
                entry.records.push(record.clone());
            }
        }
    }
}

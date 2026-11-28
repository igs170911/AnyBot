use anybot_foundation::{TimestampMs, TurnId};
use anybot_protocol::{
    ActionOutcome, EvidenceKind, EvidencePayload, EvidenceRecord, KnownTaskEventKind, TaskEvent,
    TaskEventKind,
};

const MAX_ITEMS_PER_LIST: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSummary {
    pub seen: Vec<String>,
    pub written: Vec<String>,
    pub executed: Vec<String>,
    pub tool_counts: Vec<(String, u32)>,
    pub failed_or_rejected: u32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummaryError {
    TurnNotFound,
    InvalidTimeline,
}

impl std::fmt::Display for SummaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SummaryError::TurnNotFound => f.write_str("turn not found in task events"),
            SummaryError::InvalidTimeline => f.write_str("turn timeline timestamps are invalid"),
        }
    }
}

impl std::error::Error for SummaryError {}

pub fn turn_summary(
    events: &[TaskEvent],
    turn: &TurnId,
    now: TimestampMs,
) -> Result<TurnSummary, SummaryError> {
    let segment = find_turn_segment(events, turn).ok_or(SummaryError::TurnNotFound)?;
    let mut seen: Vec<String> = Vec::new();
    let mut written: Vec<String> = Vec::new();
    let mut executed: Vec<String> = Vec::new();
    let mut tool_counts: Vec<(String, u32)> = Vec::new();
    let mut failed_or_rejected: u32 = 0;

    for event in segment {
        if let Some(record) = matching_evidence(event, turn) {
            record_payload_item(record, &mut seen, &mut written, &mut executed);
            if record.kind != EvidenceKind::Approved {
                increment_tool(&mut tool_counts, &record.tool);
            }
            if matches!(
                record.outcome,
                ActionOutcome::Failed | ActionOutcome::Rejected
            ) {
                failed_or_rejected = failed_or_rejected.saturating_add(1);
            }
        }
    }

    let elapsed_ms = compute_elapsed(segment, turn, now)?;
    Ok(TurnSummary {
        seen,
        written,
        executed,
        tool_counts,
        failed_or_rejected,
        elapsed_ms,
    })
}

fn find_turn_segment<'a>(events: &'a [TaskEvent], turn: &TurnId) -> Option<&'a [TaskEvent]> {
    let mut start = 0usize;
    let mut spans: Vec<&'a [TaskEvent]> = Vec::new();
    for (idx, event) in events.iter().enumerate() {
        if matches!(
            event.kind,
            TaskEventKind::Known(KnownTaskEventKind::TurnStarted { .. })
        ) && idx > start
        {
            if let Some(slice) = events.get(start..idx) {
                spans.push(slice);
            }
            start = idx;
        }
        if matches!(
            event.kind,
            TaskEventKind::Known(KnownTaskEventKind::TurnEnded { .. })
        ) {
            let end = idx.saturating_add(1);
            if let Some(slice) = events.get(start..end) {
                spans.push(slice);
            }
            start = end;
        }
    }
    if start < events.len()
        && let Some(slice) = events.get(start..)
    {
        spans.push(slice);
    }
    spans
        .into_iter()
        .find(|span| span.iter().any(|ev| matching_evidence(ev, turn).is_some()))
}

fn matching_evidence<'a>(event: &'a TaskEvent, turn: &TurnId) -> Option<&'a EvidenceRecord> {
    let TaskEventKind::Known(KnownTaskEventKind::EvidenceAdded { record }) = &event.kind else {
        return None;
    };
    if record.turn == *turn {
        Some(record.as_ref())
    } else {
        None
    }
}

fn record_payload_item(
    record: &EvidenceRecord,
    seen: &mut Vec<String>,
    written: &mut Vec<String>,
    executed: &mut Vec<String>,
) {
    match &record.payload {
        EvidencePayload::Seen { path, .. } => push_unique_capped(seen, path),
        EvidencePayload::Changed { path, .. } => push_unique_capped(written, path),
        EvidencePayload::Executed { command, .. } => push_unique_capped(executed, command),
        EvidencePayload::Approved { .. }
        | EvidencePayload::Rejected { .. }
        | EvidencePayload::NotExecuted { .. }
        | EvidencePayload::Relayed { .. } => {}
    }
}

fn push_unique_capped(list: &mut Vec<String>, item: &str) {
    if item.is_empty() || list.len() >= MAX_ITEMS_PER_LIST {
        return;
    }
    if !list.iter().any(|existing| existing == item) {
        list.push(item.to_owned());
    }
}

fn increment_tool(counts: &mut Vec<(String, u32)>, tool: &str) {
    if let Some((_, count)) = counts.iter_mut().find(|(name, _)| name == tool) {
        *count = count.saturating_add(1);
    } else {
        counts.push((tool.to_owned(), 1));
    }
}

fn compute_elapsed(
    segment: &[TaskEvent],
    turn: &TurnId,
    now: TimestampMs,
) -> Result<u64, SummaryError> {
    let first = segment
        .iter()
        .filter_map(|ev| step_start_at(ev, turn))
        .min()
        .ok_or(SummaryError::TurnNotFound)?;
    let finished = segment.iter().any(|ev| {
        matches!(
            ev.kind,
            TaskEventKind::Known(KnownTaskEventKind::TurnEnded { .. })
        )
    });
    let end = if finished {
        segment
            .iter()
            .filter_map(|ev| step_end_at(ev, turn))
            .max()
            .unwrap_or(first)
    } else {
        now.millis()
    };
    if end < first {
        return Err(SummaryError::InvalidTimeline);
    }
    Ok(end.saturating_sub(first))
}

fn step_start_at(event: &TaskEvent, turn: &TurnId) -> Option<u64> {
    let TaskEventKind::Known(kind) = &event.kind else {
        return None;
    };
    if let KnownTaskEventKind::GateResult { .. }
    | KnownTaskEventKind::ToolIntent { .. }
    | KnownTaskEventKind::ToolStarted { .. } = kind
    {
        return Some(event.at_ms.millis());
    }
    if let KnownTaskEventKind::EvidenceAdded { record } = kind
        && record.turn == *turn
    {
        return Some(record.at.millis());
    }
    None
}

fn step_end_at(event: &TaskEvent, turn: &TurnId) -> Option<u64> {
    let TaskEventKind::Known(kind) = &event.kind else {
        return None;
    };
    if let KnownTaskEventKind::EvidenceAdded { record } = kind
        && record.turn == *turn
    {
        return Some(record.at.millis());
    }
    if let KnownTaskEventKind::ToolCompleted { .. }
    | KnownTaskEventKind::ToolFailed { .. }
    | KnownTaskEventKind::ToolNotRun { .. }
    | KnownTaskEventKind::ActionVerified { .. }
    | KnownTaskEventKind::GateResult { .. } = kind
    {
        return Some(event.at_ms.millis());
    }
    None
}

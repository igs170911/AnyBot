use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use anybot_protocol::{TaskEvent, TaskEventKind};

use crate::task_engine::advance;
use crate::task_view::{TaskView, initial_view};

pub const SNAPSHOT_FORMAT_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayGap {
    pub missing: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub format_version: u32,
    pub through_seq: u64,
    pub view: TaskView,
}

pub fn fold(view: &mut TaskView, events: &[TaskEvent], seen: &mut BTreeSet<String>) {
    for event in events {
        if !seen.insert(event.event_id.as_str()) {
            continue;
        }
        if let TaskEventKind::Known(known) = &event.kind {
            crate::task_budget::note_time(view, event.at_ms, known);
            advance(view, known);
        }
    }
}

#[must_use]
pub fn missing_seqs(events: &[TaskEvent]) -> Vec<u64> {
    missing_seqs_from(events, 1)
}

#[must_use]
pub fn missing_seqs_from(events: &[TaskEvent], start: u64) -> Vec<u64> {
    let present: BTreeSet<u64> = events.iter().map(|event| event.seq).collect();
    let Some(max) = present.iter().next_back().copied() else {
        return Vec::new();
    };
    (start..=max).filter(|seq| !present.contains(seq)).collect()
}

pub fn replay(events: &[TaskEvent]) -> Result<TaskView, ReplayGap> {
    let missing = missing_seqs(events);
    if !missing.is_empty() {
        return Err(ReplayGap { missing });
    }
    let mut view = initial_view();
    let mut seen = BTreeSet::new();
    fold(&mut view, events, &mut seen);
    Ok(view)
}

#[must_use]
pub fn snapshot_at(events: &[TaskEvent], through_seq: u64) -> TaskSnapshot {
    let prefix: Vec<TaskEvent> = events
        .iter()
        .filter(|event| event.seq <= through_seq)
        .cloned()
        .collect();
    let mut view = initial_view();
    let mut seen = BTreeSet::new();
    fold(&mut view, &prefix, &mut seen);
    TaskSnapshot {
        format_version: SNAPSHOT_FORMAT_VERSION,
        through_seq,
        view,
    }
}

pub fn replay_from(snapshot: &TaskSnapshot, events: &[TaskEvent]) -> Result<TaskView, ReplayGap> {
    if snapshot.format_version != SNAPSHOT_FORMAT_VERSION {
        return replay(events);
    }
    let tail: Vec<TaskEvent> = events
        .iter()
        .filter(|event| event.seq > snapshot.through_seq)
        .cloned()
        .collect();
    let missing = missing_seqs_from(&tail, snapshot.through_seq.saturating_add(1));
    if !missing.is_empty() {
        return Err(ReplayGap { missing });
    }
    let mut view = snapshot.view.clone();
    let mut seen = BTreeSet::new();
    fold(&mut view, &tail, &mut seen);
    Ok(view)
}

#[cfg(test)]
#[path = "task_replay_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "task_events_prop_tests.rs"]
mod prop_tests;

use std::future::{Future, ready};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use anybot_foundation::{BotId, TaskId};
use anybot_protocol::{KnownTaskEventKind, TaskEventKind};
use anybot_runner::{
    Appended, Halt, NewTaskEvent, Record, RecordEntry, RecordError, Stored, StoredBody,
    task_event_of,
};

pub use crate::mem_raw::{MemoryRaw, share_raw};

type Probe = Arc<dyn Fn() -> usize + Send + Sync>;
pub type AfterAppend = Arc<dyn Fn(&Stored) + Send + Sync>;

#[derive(Default)]
pub struct MemoryRecord {
    rows: Mutex<Vec<(Stored, usize)>>,
    task_seq: Mutex<std::collections::BTreeMap<TaskId, u64>>,
    probe: Option<Probe>,
    after_append: Mutex<Option<AfterAppend>>,
    fail_on: Option<&'static str>,
}

impl Record for MemoryRecord {
    fn append(
        &self,
        entry: RecordEntry,
    ) -> Pin<Box<dyn Future<Output = Result<Stored, RecordError>> + Send + '_>> {
        Box::pin(ready(self.append_sync(entry)))
    }

    fn append_unless_halted(
        &self,
        entry: RecordEntry,
        turn_started: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Appended, RecordError>> + Send + '_>> {
        Box::pin(ready(self.append_unless_halted_sync(entry, turn_started)))
    }

    fn conversation(
        &self,
        bot: BotId,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Stored>, RecordError>> + Send + '_>> {
        let rows = self.snapshot();
        let kept: Vec<Stored> = rows
            .into_iter()
            .filter(move |row| match &row.body {
                StoredBody::UserSaid(line) => line.bot == bot,
                StoredBody::Task { .. } => true,
            })
            .collect();
        Box::pin(ready(Ok(kept)))
    }

    fn task_events(
        &self,
        task: TaskId,
    ) -> Pin<
        Box<dyn Future<Output = Result<Vec<anybot_protocol::TaskEvent>, RecordError>> + Send + '_>,
    > {
        Box::pin(ready(Ok(self.task_events_of(task))))
    }
}

impl MemoryRecord {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_probe(mut self, probe: Probe) -> Self {
        self.probe = Some(probe);
        self
    }

    #[must_use]
    pub fn fail_on_kind(kind_name: &'static str) -> Self {
        Self {
            fail_on: Some(kind_name),
            ..Self::default()
        }
    }

    pub fn set_after_append(&self, hook: Option<AfterAppend>) {
        if let Ok(mut guard) = self.after_append.lock() {
            *guard = hook;
        }
    }

    pub fn append_sync(&self, entry: RecordEntry) -> Result<Stored, RecordError> {
        let stored = self.insert_locked(entry, None)?;
        if let Appended::Stored(row) = stored {
            return Ok(*row);
        }
        Err(RecordError {
            reason: "unexpected halt".to_owned(),
        })
    }

    pub fn append_unless_halted_sync(
        &self,
        entry: RecordEntry,
        turn_started: u64,
    ) -> Result<Appended, RecordError> {
        self.insert_locked(entry, Some(turn_started))
    }

    fn insert_locked(
        &self,
        entry: RecordEntry,
        turn_started: Option<u64>,
    ) -> Result<Appended, RecordError> {
        let stored = {
            let Ok(mut rows) = self.rows.lock() else {
                return Err(RecordError {
                    reason: "locked".to_owned(),
                });
            };
            if let Some(since) = turn_started
                && let Some(halt) = halt_in_slice(&rows, entry_task(&entry), since)
            {
                return Ok(Appended::Halted(halt));
            }
            if let RecordEntry::Task(new) = &entry
                && self.fail_on == Some(new.kind.kind_name().as_str())
            {
                return Err(RecordError {
                    reason: "fail_on_kind".to_owned(),
                });
            }
            let body = match entry {
                RecordEntry::UserSaid(line) => StoredBody::UserSaid(line),
                RecordEntry::Task(new) => self.stored_task(&new),
            };
            let probe_value = match &self.probe {
                Some(f) => f(),
                None => 0,
            };
            let stream_seq = match rows.len().checked_add(1) {
                Some(value) => u64::try_from(value).unwrap_or(u64::MAX),
                None => u64::MAX,
            };
            let stored = Stored { stream_seq, body };
            rows.push((stored.clone(), probe_value));
            stored
        };
        let hook = self.after_append.lock().ok().and_then(|g| g.clone());
        if let Some(f) = hook {
            f(&stored);
        }
        Ok(Appended::Stored(Box::new(stored)))
    }

    #[must_use]
    pub fn entries(&self) -> Vec<(Stored, usize)> {
        let Ok(rows) = self.rows.lock() else {
            return Vec::new();
        };
        rows.clone()
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<Stored> {
        self.entries().into_iter().map(|(row, _p)| row).collect()
    }

    #[must_use]
    pub fn task_events_of(&self, task: TaskId) -> Vec<anybot_protocol::TaskEvent> {
        let rows = self.snapshot();
        rows.into_iter()
            .filter_map(move |row| match row.body {
                StoredBody::Task {
                    task: owner, event, ..
                } if owner == task => Some(event),
                StoredBody::Task { .. } | StoredBody::UserSaid(_) => None,
            })
            .collect()
    }

    fn stored_task(&self, new: &NewTaskEvent) -> StoredBody {
        let seq = self.next_task_seq(new.task);
        StoredBody::Task {
            task: new.task,
            turn: new.turn,
            event: task_event_of(new, seq),
        }
    }

    fn next_task_seq(&self, task: TaskId) -> u64 {
        let Ok(mut map) = self.task_seq.lock() else {
            return 1;
        };
        let next = map.get(&task).map_or(1, |v| v.checked_add(1).unwrap_or(1));
        map.insert(task, next);
        next
    }

    pub fn clear(&self) {
        if let Ok(mut rows) = self.rows.lock() {
            rows.clear();
        }
        if let Ok(mut map) = self.task_seq.lock() {
            map.clear();
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        let Ok(rows) = self.rows.lock() else {
            return 0;
        };
        rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn entry_task(entry: &RecordEntry) -> TaskId {
    match entry {
        RecordEntry::UserSaid(line) => line.task,
        RecordEntry::Task(new) => new.task,
    }
}

fn halt_in_slice(rows: &[(Stored, usize)], task: TaskId, turn_started: u64) -> Option<Halt> {
    let mut found: Option<Halt> = None;
    for (row, _) in rows {
        if row.stream_seq <= turn_started {
            continue;
        }
        let StoredBody::Task {
            task: owner, event, ..
        } = &row.body
        else {
            continue;
        };
        if *owner != task {
            continue;
        }
        let TaskEventKind::Known(kind) = &event.kind else {
            continue;
        };
        if matches!(kind, KnownTaskEventKind::TaskCancelled { .. }) {
            return Some(Halt::Cancelled);
        }
        if matches!(kind, KnownTaskEventKind::TaskStopped) {
            found = Some(Halt::Stopped);
        }
    }
    found
}

#[must_use]
pub fn share_record(record: &Arc<MemoryRecord>) -> Arc<dyn Record> {
    let shared: Arc<MemoryRecord> = Arc::clone(record);
    shared
}

#[cfg(test)]
#[path = "mem_store_tests.rs"]
mod tests;

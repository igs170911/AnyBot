use std::future::ready;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use anybot_foundation::{BotId, ContentDigest, TaskId};
use anybot_runner::{
    NewTaskEvent, RawStore, RawStoreError, Record, RecordEntry, RecordError, Stored, StoredBody,
    task_event_of,
};
use std::future::Future;

type Probe = Arc<dyn Fn() -> usize + Send + Sync>;

#[derive(Default)]
pub struct MemoryRecord {
    rows: Mutex<Vec<(Stored, usize)>>,
    task_seq: Mutex<std::collections::BTreeMap<TaskId, u64>>,
    probe: Option<Probe>,
    fail_on: Option<&'static str>,
}

impl Record for MemoryRecord {
    fn append(
        &self,
        entry: RecordEntry,
    ) -> Pin<Box<dyn Future<Output = Result<Stored, RecordError>> + Send + '_>> {
        let Ok(mut rows) = self.rows.lock() else {
            return Box::pin(ready(Err(RecordError {
                reason: "locked".to_owned(),
            })));
        };
        if let RecordEntry::Task(new) = &entry
            && self.fail_on == Some(new.kind.kind_name().as_str())
        {
            return Box::pin(ready(Err(RecordError {
                reason: "fail_on_kind".to_owned(),
            })));
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
        Box::pin(ready(Ok(stored)))
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
        let kept: Vec<anybot_protocol::TaskEvent> = rows
            .into_iter()
            .filter_map(move |row| match row.body {
                StoredBody::Task {
                    task: owner, event, ..
                } => {
                    if owner == task {
                        Some(event)
                    } else {
                        None
                    }
                }
                StoredBody::UserSaid(_) => None,
            })
            .collect();
        kept
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
        let Ok(mut rows) = self.rows.lock() else {
            return;
        };
        rows.clear();
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

#[must_use]
pub fn share_record(record: &Arc<MemoryRecord>) -> Arc<dyn Record> {
    let shared: Arc<MemoryRecord> = Arc::clone(record);
    shared
}

#[derive(Default)]
pub struct MemoryRaw {
    blobs: Mutex<Vec<Vec<u8>>>,
}

impl MemoryRaw {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<(String, Vec<u8>)> {
        self.items()
            .into_iter()
            .map(|(d, v)| (d.hex().to_owned(), v))
            .collect()
    }

    #[must_use]
    pub fn items(&self) -> Vec<(ContentDigest, Vec<u8>)> {
        let Ok(blobs) = self.blobs.lock() else {
            return Vec::new();
        };
        blobs
            .iter()
            .map(|b| (ContentDigest::sha256(b), b.clone()))
            .collect()
    }
}

impl RawStore for MemoryRaw {
    fn put(
        &self,
        bytes: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<ContentDigest, RawStoreError>> + Send + '_>> {
        let digest = ContentDigest::sha256(&bytes);
        if let Ok(mut blobs) = self.blobs.lock() {
            blobs.push(bytes);
        }
        Box::pin(ready(Ok(digest)))
    }

    fn get(
        &self,
        digest: ContentDigest,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>, RawStoreError>> + Send + '_>> {
        let Ok(blobs) = self.blobs.lock() else {
            return Box::pin(ready(Ok(None)));
        };
        for blob in &*blobs {
            if ContentDigest::sha256(blob) == digest {
                return Box::pin(ready(Ok(Some(blob.clone()))));
            }
        }
        Box::pin(ready(Ok(None)))
    }
}

#[must_use]
pub fn share_raw(raw: &Arc<MemoryRaw>) -> Arc<dyn RawStore> {
    let shared: Arc<MemoryRaw> = Arc::clone(raw);
    shared
}

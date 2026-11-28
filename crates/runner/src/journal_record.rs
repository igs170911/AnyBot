use std::sync::Arc;

use anybot_foundation::{BotId, PinBoxFuture, TaskId};
use anybot_protocol::TaskEvent;
use anybot_store_client::{JournalAppend, Store, bot_stream};

use crate::journal_codec::{build_entry, merge_conversation, merge_task_events, outbox_stream_seq};
use crate::journal_mask::{BasicSecretMasker, SecretMasker};
use crate::record::{
    Appended, Halt, Record, RecordEntry, RecordError, Stored, StoredBody, task_event_of,
};

pub struct JournalRecord {
    store: Store,
    masker: Arc<dyn SecretMasker>,
}

impl JournalRecord {
    #[must_use]
    pub fn new(store: Store) -> Self {
        Self {
            store,
            masker: Arc::new(BasicSecretMasker),
        }
    }

    #[must_use]
    pub fn with_masker(store: Store, masker: Arc<dyn SecretMasker>) -> Self {
        Self { store, masker }
    }

    #[must_use]
    pub fn store(&self) -> &Store {
        &self.store
    }

    async fn stored_from_append(
        &self,
        entry: RecordEntry,
        local_id: i64,
    ) -> Result<Stored, RecordError> {
        let stream_seq = outbox_stream_seq(local_id)?;
        let body = match entry {
            RecordEntry::UserSaid(line) => StoredBody::UserSaid(line),
            RecordEntry::Task(new) => {
                let events = self.task_events(new.task).await?;
                let seq = u64::try_from(events.len()).unwrap_or(1);
                StoredBody::Task {
                    task: new.task,
                    turn: new.turn,
                    event: task_event_of(&new, seq),
                }
            }
        };
        Ok(Stored { stream_seq, body })
    }
}

impl Record for JournalRecord {
    fn append(&self, entry: RecordEntry) -> PinBoxFuture<'_, Result<Stored, RecordError>> {
        Box::pin(async move {
            let built = build_entry(&entry, &*self.masker)?;
            let row = self
                .store
                .journal_append(&built)
                .await
                .map_err(|e| store_err(&e))?;
            self.stored_from_append(entry, row.local_id).await
        })
    }

    fn append_unless_halted(
        &self,
        entry: RecordEntry,
        turn_started: u64,
    ) -> PinBoxFuture<'_, Result<Appended, RecordError>> {
        Box::pin(async move {
            let built = build_entry(&entry, &*self.masker)?;
            let task_id = built.task_id.clone().unwrap_or_default();
            let started = i64::try_from(turn_started).unwrap_or(i64::MAX);
            let outcome = self
                .store
                .journal_append_unless_halted(&built, started, &task_id)
                .await
                .map_err(|e| store_err(&e))?;
            match outcome {
                JournalAppend::Stopped => Ok(Appended::Halted(Halt::Stopped)),
                JournalAppend::Cancelled => Ok(Appended::Halted(Halt::Cancelled)),
                JournalAppend::Stored(row) => {
                    let stored = self.stored_from_append(entry, row.local_id).await?;
                    Ok(Appended::Stored(Box::new(stored)))
                }
            }
        })
    }

    fn conversation(&self, bot: BotId) -> PinBoxFuture<'_, Result<Vec<Stored>, RecordError>> {
        Box::pin(async move {
            let stream = bot_stream(&bot.as_str());
            let replica = self
                .store
                .records(&stream)
                .await
                .map_err(|e| store_err(&e))?;
            let pending = self
                .store
                .journal_stream_rows(&stream)
                .await
                .map_err(|e| store_err(&e))?;
            merge_conversation(&replica, &pending)
        })
    }

    fn task_events(&self, task: TaskId) -> PinBoxFuture<'_, Result<Vec<TaskEvent>, RecordError>> {
        Box::pin(async move {
            let pending = self
                .store
                .journal_task_rows(&task.as_str())
                .await
                .map_err(|e| store_err(&e))?;
            let streams = self
                .store
                .replica_streams()
                .await
                .map_err(|e| store_err(&e))?;
            let mut replica = Vec::new();
            for stream in streams {
                if stream.starts_with("bot/") {
                    let rows = self
                        .store
                        .records(&stream)
                        .await
                        .map_err(|e| store_err(&e))?;
                    replica.extend(rows);
                }
            }
            merge_task_events(task, &replica, &pending)
        })
    }

    fn trim_turn(&self, task: TaskId, turn: u32) -> PinBoxFuture<'_, Result<(), RecordError>> {
        Box::pin(async move {
            self.store
                .journal_trim(&task.as_str(), i64::from(turn))
                .await
                .map_err(|e| store_err(&e))
        })
    }
}

fn store_err(err: &anybot_store_client::StoreError) -> RecordError {
    RecordError {
        reason: err.to_string(),
    }
}

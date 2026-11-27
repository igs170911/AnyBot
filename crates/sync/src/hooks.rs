use std::sync::Arc;

use anybot_store_client::ACCOUNT_STREAM;

use crate::core::Core;
use crate::error::SyncError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamMark {
    pub stream: String,
    pub seq: i64,
    pub record_id: String,
}

#[derive(Clone)]
pub struct SyncHooks {
    core: Arc<Core>,
}

pub(crate) fn local_key(stream: &str) -> String {
    if stream == ACCOUNT_STREAM || stream.starts_with("account/") {
        ACCOUNT_STREAM.to_owned()
    } else {
        stream.to_owned()
    }
}

impl SyncHooks {
    pub(crate) fn new(core: Arc<Core>) -> Self {
        SyncHooks { core }
    }

    pub async fn server_progress(&self, marks: &[StreamMark]) -> Result<(), SyncError> {
        for mark in marks {
            self.compare(mark).await?;
        }
        Ok(())
    }

    pub async fn server_has_news(&self, mark: &StreamMark) -> Result<(), SyncError> {
        let stream = local_key(&mark.stream);
        let local = self.core.store.progress(&stream).await?;
        if mark.seq > local.seq {
            self.core.wanted.pull(&stream);
            self.core.wake.notify_one();
        }
        Ok(())
    }

    pub fn pull_now(&self, stream: &str) {
        self.core.wanted.pull(&local_key(stream));
        self.core.wake.notify_one();
    }

    pub fn connection_gained(&self) {
        self.core.note_connected();
        self.core.retry_now.notify_one();
        self.core.wake.notify_one();
    }

    pub fn connection_lost(&self, reason: &str) -> Result<(), SyncError> {
        self.core.note_failure(reason)
    }

    pub fn retry_now(&self) {
        self.core.link.retry.notify_waiters();
        self.core.retry_now.notify_one();
        self.core.wake.notify_one();
    }

    async fn compare(&self, mark: &StreamMark) -> Result<(), SyncError> {
        let stream = local_key(&mark.stream);
        let local = self.core.store.progress(&stream).await?;
        let same = self.same_record(&stream, mark, local.seq).await?;
        if mark.seq > local.seq || (mark.seq < local.seq && same) {
            self.core.wanted.pull(&stream);
        } else if !same {
            self.core.wanted.resync(&stream);
        }
        self.core.wake.notify_one();
        Ok(())
    }

    async fn same_record(
        &self,
        stream: &str,
        mark: &StreamMark,
        local_seq: i64,
    ) -> Result<bool, SyncError> {
        if mark.seq == 0 {
            return Ok(true);
        }
        if mark.seq > local_seq {
            return Ok(true);
        }
        let held = self.core.store.record_at(stream, mark.seq).await?;
        Ok(held.is_some_and(|record| record.record_id == mark.record_id))
    }
}

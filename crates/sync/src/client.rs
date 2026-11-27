use std::sync::Arc;

use anybot_foundation::{Clock, RecordId, Secret};
use anybot_store_client::{
    ACCOUNT_STREAM, LocalRecord, NewOutbox, OpenOutcome, OutboxRow, Recovery, Store, bot_stream,
};
use serde_json::Value;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::config::SyncConfig;
use crate::core::Core;
use crate::error::SyncError;
use crate::hooks::SyncHooks;
use crate::http_wire::HttpWire;
use crate::link_task::{LinkParts, run_keepalive, run_link, run_slowness, run_status_mirror};
use crate::reconnect_http::build_sync_http;
use crate::reconnect_random::RandomSource;
use crate::sse_session::SseTarget;
use crate::status::{ReplicaNotice, SyncStatus};
use crate::wire::Wire;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamTarget {
    Account,
    Bot(String),
}

impl StreamTarget {
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            StreamTarget::Account => ACCOUNT_STREAM.to_owned(),
            StreamTarget::Bot(id) => bot_stream(id),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewRecord {
    pub stream: StreamTarget,
    pub kind: String,
    pub content: Value,
    pub task_id: Option<String>,
}

struct TaskGuard(JoinHandle<()>);

impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub struct SyncClient {
    pub(crate) core: Arc<Core>,
    tasks: Vec<TaskGuard>,
}

impl SyncClient {
    pub async fn start(config: SyncConfig) -> Result<SyncClient, SyncError> {
        let wire = HttpWire::new(
            &config.server_url,
            &config.credential,
            &config.client_version,
            &config.trusted,
            config.proxy.as_deref(),
            Arc::clone(&config.clock),
        )
        .map_err(SyncError::Config)?;
        let http = build_sync_http(
            &config.trusted,
            config.proxy.as_deref(),
            Arc::clone(&config.clock),
        )
        .map_err(SyncError::Config)?;
        let target = SseTarget {
            base_url: config.server_url.clone(),
            credential: Secret::from_untrusted(config.credential.expose().to_owned()),
            client_version: config.client_version.clone(),
        };
        let opened = open_store(&config).await?;
        let mut client =
            SyncClient::assemble(opened, Arc::new(wire), config.clock, config.random).await?;
        client.start_link(LinkParts { http, target });
        Ok(client)
    }

    fn start_link(&mut self, parts: LinkParts) {
        let core = &self.core;
        self.tasks.extend([
            TaskGuard(tokio::spawn(run_link(Arc::clone(core), parts))),
            TaskGuard(tokio::spawn(run_keepalive(Arc::clone(core)))),
            TaskGuard(tokio::spawn(run_slowness(Arc::clone(core)))),
        ]);
    }

    pub async fn start_with_wire(
        data_dir: &std::path::Path,
        wire: Arc<dyn Wire>,
        clock: Arc<dyn Clock>,
        random: Arc<dyn RandomSource>,
    ) -> Result<SyncClient, SyncError> {
        let stamp = stamp_of(clock.as_ref())?;
        let opened = Store::open(data_dir, stamp).await?;
        SyncClient::assemble(opened, wire, clock, random).await
    }

    async fn assemble(
        opened: OpenOutcome,
        wire: Arc<dyn Wire>,
        clock: Arc<dyn Clock>,
        random: Arc<dyn RandomSource>,
    ) -> Result<SyncClient, SyncError> {
        let mut status = SyncStatus::initial();
        status.replica_notice = opened.recovery.as_ref().map(notice_of);
        let core = Arc::new(Core::new(opened.store, wire, clock, random, status));
        core.refresh_counts().await?;
        want_everything_known(&core).await?;
        let worker = tokio::spawn(crate::worker::run(Arc::clone(&core)));
        let mirror = tokio::spawn(run_status_mirror(Arc::clone(&core)));
        core.wake.notify_one();
        Ok(SyncClient {
            core,
            tasks: vec![TaskGuard(worker), TaskGuard(mirror)],
        })
    }

    pub async fn close(self) {
        for task in &self.tasks {
            task.0.abort();
        }
        self.core.store.close().await;
    }

    #[must_use]
    pub fn hooks(&self) -> SyncHooks {
        SyncHooks::new(Arc::clone(&self.core))
    }

    pub fn pull_now(&self, stream: &StreamTarget) {
        self.hooks().pull_now(&stream.key());
    }

    pub async fn add_record(&self, record: NewRecord) -> Result<String, SyncError> {
        let record_id = RecordId::generate(self.core.clock.as_ref()).as_str();
        let item = NewOutbox {
            stream: record.stream.key(),
            record_id: record_id.clone(),
            kind: record.kind,
            content: record.content,
            created_at_ms: self.core.now_ms()?,
            task_id: record.task_id,
            offline_at_create: !self.core.is_connected(),
        };
        self.core.store.enqueue(&item).await?;
        self.core.refresh_counts().await?;
        self.core.wake.notify_one();
        Ok(record_id)
    }

    pub async fn cancel_unsent(&self, record_id: &str) -> Result<Option<OutboxRow>, SyncError> {
        let taken = self.core.store.outbox_cancel_unsent(record_id).await?;
        self.core.refresh_counts().await?;
        Ok(taken)
    }

    pub async fn take_back_rejected(
        &self,
        record_id: &str,
    ) -> Result<Option<OutboxRow>, SyncError> {
        let taken = self.core.store.outbox_take_back_rejected(record_id).await?;
        self.core.refresh_counts().await?;
        Ok(taken)
    }

    pub async fn can_cancel(&self, record_id: &str) -> Result<bool, SyncError> {
        let rows = self.core.store.outbox_all().await?;
        Ok(rows.iter().any(|row| {
            row.record_id == record_id
                && row.attempted_at_ms.is_none()
                && row.state == anybot_store_client::OutboxState::Pending
        }))
    }

    pub async fn outbox(&self) -> Result<Vec<OutboxRow>, SyncError> {
        Ok(self.core.store.outbox_all().await?)
    }

    pub async fn records(&self, stream: &StreamTarget) -> Result<Vec<LocalRecord>, SyncError> {
        Ok(self.core.store.records(&stream.key()).await?)
    }

    #[must_use]
    pub fn status(&self) -> SyncStatus {
        self.core.status.get()
    }

    #[must_use]
    pub fn watch_status(&self) -> watch::Receiver<SyncStatus> {
        self.core.status.subscribe()
    }
}

pub(crate) async fn want_everything_known(core: &Core) -> Result<(), SyncError> {
    core.wanted.pull(ACCOUNT_STREAM);
    for stream in core.store.replica_streams().await? {
        core.wanted.pull(&stream);
    }
    Ok(())
}

async fn open_store(config: &SyncConfig) -> Result<OpenOutcome, SyncError> {
    let stamp = stamp_of(config.clock.as_ref())?;
    Ok(Store::open(&config.data_dir, stamp).await?)
}

fn stamp_of(clock: &dyn Clock) -> Result<i64, SyncError> {
    i64::try_from(clock.now_ms().millis())
        .map_err(|e| SyncError::Config(format!("clock out of range: {e}")))
}

fn notice_of(recovery: &Recovery) -> ReplicaNotice {
    ReplicaNotice {
        code: "replica-recovered".to_owned(),
        reason_code: recovery.reason.code().to_owned(),
        detail: recovery.reason.detail().to_owned(),
        moved_to: recovery.moved_to.display().to_string(),
        rescued_outbox: recovery.rescued_outbox,
        unrecovered_outbox: recovery.unrecovered_outbox.len(),
        outbox_unreadable: recovery.outbox_unreadable,
    }
}

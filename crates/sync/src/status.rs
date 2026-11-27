use tokio::sync::watch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Unreachable { since_ms: i64, reason: String },
    NeedLogin,
    VersionTooOld,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaNotice {
    pub code: String,
    pub reason_code: String,
    pub detail: String,
    pub moved_to: String,
    pub rescued_outbox: usize,
    pub unrecovered_outbox: usize,
    pub outbox_unreadable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResyncNotice {
    pub stream: String,
    pub at_ms: i64,
    pub resent: usize,
    pub server_written_not_resent: usize,
    pub purged_not_resent: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStatus {
    pub connection: ConnectionState,
    pub pending: u64,
    pub rejected: u64,
    pub last_sync_ms: Option<i64>,
    pub replica_notice: Option<ReplicaNotice>,
    pub unrecovered_kinds: u64,
    pub resyncs: Vec<ResyncNotice>,
    pub local_error: Option<String>,
}

impl SyncStatus {
    #[must_use]
    pub fn initial() -> Self {
        SyncStatus {
            connection: ConnectionState::Connecting,
            pending: 0,
            rejected: 0,
            last_sync_ms: None,
            replica_notice: None,
            unrecovered_kinds: 0,
            resyncs: Vec::new(),
            local_error: None,
        }
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.connection == ConnectionState::Connected
    }
}

pub(crate) struct StatusCell {
    sender: watch::Sender<SyncStatus>,
}

impl StatusCell {
    pub(crate) fn new(initial: SyncStatus) -> Self {
        let (sender, _watcher) = watch::channel(initial);
        StatusCell { sender }
    }

    pub(crate) fn get(&self) -> SyncStatus {
        self.sender.borrow().clone()
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<SyncStatus> {
        self.sender.subscribe()
    }

    pub(crate) fn update(&self, change: impl FnOnce(&mut SyncStatus)) {
        self.sender.send_modify(change);
    }
}

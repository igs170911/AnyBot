use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use anybot_foundation::Clock;
use anybot_store_client::Store;
use tokio::sync::Notify;

use crate::error::SyncError;
use crate::outbox::retry_delay_secs;
use crate::random::{RandomSource, span_ms};
use crate::status::{ConnectionState, StatusCell, SyncStatus};
use crate::wanted::Wanted;
use crate::wire::{Wire, WireReply};

const JITTER_LOW_MS: u64 = 500;
const JITTER_HIGH_MS: u64 = 2_000;

#[derive(Debug, Default)]
pub(crate) struct Pace {
    pub last_send_ms: Option<i64>,
    pub failures: u64,
    pub offset_ms: Option<i64>,
}

pub(crate) struct Core {
    pub store: Store,
    pub wire: Arc<dyn Wire>,
    pub clock: Arc<dyn Clock>,
    pub random: Arc<dyn RandomSource>,
    pub status: StatusCell,
    pub wake: Notify,
    pub retry_now: Notify,
    pub wanted: Wanted,
    pace: Mutex<Pace>,
}

impl Core {
    pub(crate) fn new(
        store: Store,
        wire: Arc<dyn Wire>,
        clock: Arc<dyn Clock>,
        random: Arc<dyn RandomSource>,
        status: SyncStatus,
    ) -> Self {
        Core {
            store,
            wire,
            clock,
            random,
            status: StatusCell::new(status),
            wake: Notify::new(),
            retry_now: Notify::new(),
            wanted: Wanted::default(),
            pace: Mutex::new(Pace::default()),
        }
    }

    pub(crate) fn pace(&self) -> MutexGuard<'_, Pace> {
        match self.pace.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub(crate) fn now_ms(&self) -> Result<i64, SyncError> {
        i64::try_from(self.clock.now_ms().millis())
            .map_err(|e| SyncError::Config(format!("clock out of range: {e}")))
    }

    pub(crate) fn is_connected(&self) -> bool {
        self.status.get().is_connected()
    }

    pub(crate) fn halted(&self) -> bool {
        matches!(
            self.status.get().connection,
            ConnectionState::NeedLogin | ConnectionState::VersionTooOld
        )
    }

    pub(crate) async fn refresh_counts(&self) -> Result<(), SyncError> {
        let (pending, rejected) = self.store.outbox_counts().await?;
        let waiting = u64::try_from(pending).map_err(|e| SyncError::Config(e.to_string()))?;
        let refused = u64::try_from(rejected).map_err(|e| SyncError::Config(e.to_string()))?;
        self.status.update(|status| {
            status.pending = waiting;
            status.rejected = refused;
        });
        Ok(())
    }

    pub(crate) fn note_reply(&self, reply: &WireReply) -> Result<(), SyncError> {
        let received = self.now_ms()?;
        {
            let mut pace = self.pace();
            pace.failures = 0;
            if let Some(server_time) = reply.server_time_ms {
                pace.offset_ms = Some(server_time.saturating_sub(received));
            }
        }
        self.status.update(|status| {
            if !matches!(
                status.connection,
                ConnectionState::NeedLogin | ConnectionState::VersionTooOld
            ) {
                status.connection = ConnectionState::Connected;
            }
        });
        Ok(())
    }

    pub(crate) fn note_connected(&self) {
        self.pace().failures = 0;
        self.status.update(|status| {
            if !matches!(
                status.connection,
                ConnectionState::NeedLogin | ConnectionState::VersionTooOld
            ) {
                status.connection = ConnectionState::Connected;
            }
        });
    }

    pub(crate) fn note_failure(&self, reason: &str) -> Result<(), SyncError> {
        let at = self.now_ms()?;
        {
            let mut pace = self.pace();
            pace.failures = pace.failures.saturating_add(1);
            pace.offset_ms = None;
        }
        let reason = reason.to_owned();
        self.status.update(|status| {
            if let ConnectionState::Unreachable { since_ms, .. } = status.connection {
                status.connection = ConnectionState::Unreachable { since_ms, reason };
            } else if !matches!(
                status.connection,
                ConnectionState::NeedLogin | ConnectionState::VersionTooOld
            ) {
                status.connection = ConnectionState::Unreachable {
                    since_ms: at,
                    reason,
                };
            }
        });
        Ok(())
    }

    pub(crate) fn note_server_trouble(&self) {
        let mut pace = self.pace();
        pace.failures = pace.failures.saturating_add(1);
    }

    pub(crate) fn halt(&self, state: ConnectionState) {
        self.status.update(|status| status.connection = state);
    }

    pub(crate) fn backoff_delay(&self) -> Duration {
        let failures = self.pace().failures;
        let base_secs = retry_delay_secs(failures.saturating_sub(1));
        let jitter = span_ms(self.random.as_ref(), JITTER_LOW_MS, JITTER_HIGH_MS);
        Duration::from_millis(base_secs.saturating_mul(1_000).saturating_add(jitter))
    }
}

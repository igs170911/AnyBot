use std::sync::{Arc, Mutex};
use std::time::Duration;

use anybot_foundation::{Clock, TimestampMs};
use tokio::sync::Notify;

use crate::outbox::retry_delay_secs;
use crate::reconnect_random::{RandomSource, RandomUnavailable, jitter_ms};

pub const CONNECT_PHASE: Duration = Duration::from_secs(5);
pub const IDLE_LIMIT: Duration = Duration::from_secs(30);
pub const STABLE_AFTER: Duration = Duration::from_secs(30);

#[derive(Debug)]
struct PacerState {
    failures_in_row: u64,
    connected_at: Option<TimestampMs>,
    retry_at: Option<TimestampMs>,
}

pub struct ReconnectPacer {
    clock: Arc<dyn Clock>,
    random: Arc<dyn RandomSource>,
    state: Mutex<PacerState>,
    connected_signal: Notify,
}

impl ReconnectPacer {
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>, random: Arc<dyn RandomSource>) -> Self {
        ReconnectPacer {
            clock,
            random,
            state: Mutex::new(PacerState {
                failures_in_row: 0,
                connected_at: None,
                retry_at: None,
            }),
            connected_signal: Notify::new(),
        }
    }

    pub fn connected(&self) {
        let now = self.clock.now_ms();
        {
            let mut state = lock(&self.state);
            if state.connected_at.is_none() {
                state.connected_at = Some(now);
            }
            state.retry_at = None;
        }
        self.connected_signal.notify_waiters();
    }

    pub fn failed(&self) -> Result<Duration, RandomUnavailable> {
        let now = self.clock.now_ms();
        let jitter = jitter_ms(self.random.as_ref())?;
        let mut state = lock(&self.state);
        let stable = state.connected_at.is_some_and(|since| {
            now.millis().saturating_sub(since.millis()) > millis_of(STABLE_AFTER)
        });
        if stable {
            state.failures_in_row = 0;
        }
        let base_secs = retry_delay_secs(state.failures_in_row);
        let delay_ms = base_secs.saturating_mul(1_000).saturating_add(jitter);
        state.failures_in_row = state.failures_in_row.saturating_add(1);
        state.connected_at = None;
        state.retry_at = Some(TimestampMs::from_millis(
            now.millis().saturating_add(delay_ms),
        ));
        Ok(Duration::from_millis(delay_ms))
    }

    #[must_use]
    pub fn retry_at(&self) -> Option<TimestampMs> {
        lock(&self.state).retry_at
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        lock(&self.state).connected_at.is_some()
    }

    #[must_use]
    pub fn failures_in_row(&self) -> u64 {
        lock(&self.state).failures_in_row
    }

    pub async fn wait_turn(&self) {
        loop {
            let notified = self.connected_signal.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let Some(retry_at) = self.retry_at() else {
                return;
            };
            if retry_at <= self.clock.now_ms() {
                return;
            }
            tokio::select! {
                () = self.clock.sleep_until(retry_at) => return,
                () = notified => {}
            }
        }
    }
}

fn millis_of(span: Duration) -> u64 {
    u64::try_from(span.as_millis()).unwrap_or(u64::MAX)
}

fn lock(state: &Mutex<PacerState>) -> std::sync::MutexGuard<'_, PacerState> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
#[path = "reconnect_pacing_tests.rs"]
mod tests;

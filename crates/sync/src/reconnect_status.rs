use std::sync::{Arc, Mutex};
use std::time::Duration;

use anybot_foundation::{Clock, TimestampMs};
use tokio::sync::{Notify, watch};

use crate::sse_session::SseEnd;

pub const SLOW_AFTER: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkPhase {
    Connecting,
    Connected,
    Slow,
    Retrying,
    CredentialRejected,
    VersionTooOld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkReason {
    ConnectTimeout,
    ConnectFailed,
    Idle,
    Dropped,
    ClosedByServer,
    Rejected,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkStatus {
    pub phase: LinkPhase,
    pub reason: Option<LinkReason>,
    pub since: TimestampMs,
    pub retry_at: Option<TimestampMs>,
    pub failures_in_row: u64,
}

#[must_use]
pub fn reason_of(end: &SseEnd) -> Option<LinkReason> {
    match end {
        SseEnd::ConnectTimeout => Some(LinkReason::ConnectTimeout),
        SseEnd::ConnectFailed(_) => Some(LinkReason::ConnectFailed),
        SseEnd::Idle => Some(LinkReason::Idle),
        SseEnd::Dropped(_) => Some(LinkReason::Dropped),
        SseEnd::ClosedByServer => Some(LinkReason::ClosedByServer),
        SseEnd::Rejected { .. } => Some(LinkReason::Rejected),
        SseEnd::CredentialRejected | SseEnd::VersionTooOld | SseEnd::ReceiverGone => None,
    }
}

pub struct StatusBoard {
    clock: Arc<dyn Clock>,
    sender: watch::Sender<LinkStatus>,
    slow: Mutex<SlowState>,
    changed: Notify,
}

struct SlowState {
    in_flight: usize,
    last_bytes: TimestampMs,
    before_slow: Option<LinkStatus>,
}

pub struct RequestGuard {
    board: Arc<StatusBoard>,
}

impl RequestGuard {
    pub fn bytes_arrived(&self) {
        self.board.bytes_arrived();
    }
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.board.request_finished();
    }
}

impl StatusBoard {
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        let now = clock.now_ms();
        let first = LinkStatus {
            phase: LinkPhase::Connecting,
            reason: None,
            since: clock.now_ms(),
            retry_at: None,
            failures_in_row: 0,
        };
        let (sender, _first_receiver) = watch::channel(first);
        StatusBoard {
            clock,
            sender,
            slow: Mutex::new(SlowState {
                in_flight: 0,
                last_bytes: now,
                before_slow: None,
            }),
            changed: Notify::new(),
        }
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<LinkStatus> {
        self.sender.subscribe()
    }

    #[must_use]
    pub fn current(&self) -> LinkStatus {
        *self.sender.borrow()
    }

    pub fn connecting(&self) {
        let before = self.current();
        self.publish(LinkStatus {
            phase: LinkPhase::Connecting,
            retry_at: None,
            since: self.clock.now_ms(),
            ..before
        });
    }

    pub fn connected(&self) {
        self.publish(LinkStatus {
            phase: LinkPhase::Connected,
            reason: None,
            since: self.clock.now_ms(),
            retry_at: None,
            failures_in_row: 0,
        });
    }

    pub fn ended(&self, end: &SseEnd, retry_at: Option<TimestampMs>, failures_in_row: u64) {
        let phase = match end {
            SseEnd::CredentialRejected => LinkPhase::CredentialRejected,
            SseEnd::VersionTooOld => LinkPhase::VersionTooOld,
            SseEnd::ConnectTimeout
            | SseEnd::ConnectFailed(_)
            | SseEnd::Idle
            | SseEnd::Dropped(_)
            | SseEnd::ClosedByServer
            | SseEnd::Rejected { .. }
            | SseEnd::ReceiverGone => LinkPhase::Retrying,
        };
        self.publish(LinkStatus {
            phase,
            reason: reason_of(end),
            since: self.clock.now_ms(),
            retry_at,
            failures_in_row,
        });
    }

    #[must_use]
    pub fn request_started(self: &Arc<Self>) -> RequestGuard {
        let now = self.clock.now_ms();
        {
            let mut state = self.slow_state();
            if state.in_flight == 0 {
                state.last_bytes = now;
            }
            state.in_flight = state.in_flight.saturating_add(1);
        }
        self.changed.notify_waiters();
        RequestGuard {
            board: Arc::clone(self),
        }
    }

    fn request_finished(&self) {
        let restore = {
            let mut state = self.slow_state();
            state.in_flight = state.in_flight.saturating_sub(1);
            if state.in_flight == 0 {
                state.before_slow.take()
            } else {
                None
            }
        };
        self.leave_slow(restore);
        self.changed.notify_waiters();
    }

    pub fn bytes_arrived(&self) {
        let now = self.clock.now_ms();
        let restore = {
            let mut state = self.slow_state();
            state.last_bytes = now;
            state.before_slow.take()
        };
        self.leave_slow(restore);
        self.changed.notify_waiters();
    }

    fn leave_slow(&self, restore: Option<LinkStatus>) {
        if let Some(saved) = restore
            && self.current().phase == LinkPhase::Slow
        {
            self.publish(saved);
        }
    }

    pub async fn watch_slowness(&self) {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let Some(due) = self.slow_due() else {
                changed.await;
                continue;
            };
            if self.clock.now_ms() >= due {
                self.mark_slow();
                continue;
            }
            tokio::select! {
                () = self.clock.sleep_until(due) => {}
                () = changed => {}
            }
        }
    }

    fn slow_due(&self) -> Option<TimestampMs> {
        let phase = self.current().phase;
        if phase != LinkPhase::Connected && phase != LinkPhase::Connecting {
            return None;
        }
        let state = self.slow_state();
        (state.in_flight > 0).then(|| state.last_bytes.saturating_add_ms(SLOW_AFTER))
    }

    fn mark_slow(&self) {
        let before = self.current();
        self.slow_state().before_slow = Some(before);
        self.publish(LinkStatus {
            phase: LinkPhase::Slow,
            reason: Some(LinkReason::Slow),
            since: self.clock.now_ms(),
            ..before
        });
    }

    fn slow_state(&self) -> std::sync::MutexGuard<'_, SlowState> {
        match self.slow.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn publish(&self, status: LinkStatus) {
        self.sender.send_replace(status);
        self.changed.notify_waiters();
    }
}

#[cfg(test)]
#[path = "reconnect_status_tests.rs"]
mod tests;

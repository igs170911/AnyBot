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
    last_bytes: Mutex<TimestampMs>,
    changed: Notify,
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
            last_bytes: Mutex::new(now),
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
        self.note_bytes();
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

    pub fn bytes_arrived(&self) {
        self.note_bytes();
        let before = self.current();
        if before.phase == LinkPhase::Slow {
            self.publish(LinkStatus {
                phase: LinkPhase::Connected,
                reason: None,
                since: self.clock.now_ms(),
                ..before
            });
        } else {
            self.changed.notify_waiters();
        }
    }

    pub async fn watch_slowness(&self) {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.current().phase != LinkPhase::Connected {
                changed.await;
                continue;
            }
            let due = self.last_bytes_at().saturating_add_ms(SLOW_AFTER);
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

    fn mark_slow(&self) {
        let before = self.current();
        self.publish(LinkStatus {
            phase: LinkPhase::Slow,
            reason: Some(LinkReason::Slow),
            since: self.clock.now_ms(),
            ..before
        });
    }

    fn note_bytes(&self) {
        let now = self.clock.now_ms();
        match self.last_bytes.lock() {
            Ok(mut guard) => *guard = now,
            Err(poisoned) => *poisoned.into_inner() = now,
        }
    }

    fn last_bytes_at(&self) -> TimestampMs {
        match self.last_bytes.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => *poisoned.into_inner(),
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

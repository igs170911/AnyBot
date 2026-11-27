use std::sync::Arc;

use anybot_foundation::{Clock, TimestampMs};
use tokio::sync::watch;

use crate::sse_session::SseEnd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkPhase {
    Connecting,
    Connected,
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
}

impl StatusBoard {
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        let first = LinkStatus {
            phase: LinkPhase::Connecting,
            reason: None,
            since: clock.now_ms(),
            retry_at: None,
            failures_in_row: 0,
        };
        let (sender, _first_receiver) = watch::channel(first);
        StatusBoard { clock, sender }
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

    fn publish(&self, status: LinkStatus) {
        self.sender.send_replace(status);
    }
}

#[cfg(test)]
#[path = "reconnect_status_tests.rs"]
mod tests;

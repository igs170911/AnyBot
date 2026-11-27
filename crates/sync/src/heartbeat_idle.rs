use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::{Clock, TimestampMs};
use bytes::Bytes;

use crate::reconnect_connect::{ConnectProbe, is_connect_phase_timeout};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkFailure {
    ConnectTimeout,
    ConnectFailed(String),
    Idle,
    Broken(String),
}

#[must_use]
pub fn classify_request_error(error: &reqwest::Error) -> LinkFailure {
    if is_connect_phase_timeout(error) {
        return LinkFailure::ConnectTimeout;
    }
    if error.is_connect() {
        return LinkFailure::ConnectFailed(describe(error));
    }
    LinkFailure::Broken(describe(error))
}

fn describe(error: &reqwest::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(inner) = source {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        source = inner.source();
    }
    text
}

pub struct IdleWatcher {
    clock: Arc<dyn Clock>,
    limit: Duration,
    probe: Arc<ConnectProbe>,
    started: TimestampMs,
    last_bytes: Option<TimestampMs>,
}

impl IdleWatcher {
    #[must_use]
    pub fn start(clock: Arc<dyn Clock>, limit: Duration, probe: Arc<ConnectProbe>) -> Self {
        let started = clock.now_ms();
        IdleWatcher {
            clock,
            limit,
            probe,
            started,
            last_bytes: None,
        }
    }

    pub fn touch(&mut self) {
        self.last_bytes = Some(self.clock.now_ms());
    }

    #[must_use]
    pub fn deadline(&self) -> TimestampMs {
        let connected = self.probe.last().filter(|at| *at >= self.started);
        let latest = [Some(self.started), connected, self.last_bytes]
            .into_iter()
            .flatten()
            .max()
            .unwrap_or(self.started);
        latest.saturating_add_ms(self.limit)
    }

    pub async fn expired(&self) {
        loop {
            let deadline = self.deadline();
            if self.clock.now_ms() >= deadline {
                return;
            }
            self.clock.sleep_until(deadline).await;
        }
    }

    pub async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, LinkFailure> {
        tokio::select! {
            sent = request.send() => sent.map_err(|error| classify_request_error(&error)),
            () = self.expired() => Err(LinkFailure::Idle),
        }
    }

    pub async fn next_chunk(
        &mut self,
        response: &mut reqwest::Response,
    ) -> Result<Option<Bytes>, LinkFailure> {
        let chunk = tokio::select! {
            read = response.chunk() => read.map_err(|error| classify_request_error(&error))?,
            () = self.expired() => return Err(LinkFailure::Idle),
        };
        if chunk.is_some() {
            self.touch();
        }
        Ok(chunk)
    }
}

#[cfg(test)]
#[path = "heartbeat_idle_tests.rs"]
mod tests;

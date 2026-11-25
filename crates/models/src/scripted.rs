use std::sync::{Arc, Mutex};

use crate::transport::{HttpBytes, Posted, PostedOutcome, Poster, TransportFail};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScriptStep {
    pub status: u16,
    pub body: String,
    pub fail: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct SentRow {
    pub url: String,
    pub body: String,
}

impl SentRow {
    pub(crate) fn shows_destination(&self) -> &str {
        &self.url
    }

    pub(crate) fn carries(&self) -> &str {
        &self.body
    }
}

pub(crate) struct ScriptPoster {
    steps: Mutex<Vec<ScriptStep>>,
    sent: Mutex<Vec<SentRow>>,
    on_send: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}

impl ScriptPoster {
    pub(crate) fn new(steps: Vec<ScriptStep>) -> Arc<Self> {
        Arc::new(Self {
            steps: Mutex::new(steps),
            sent: Mutex::new(Vec::new()),
            on_send: Mutex::new(None),
        })
    }

    pub(crate) fn arm_send(&self, hook: impl Fn() + Send + Sync + 'static) {
        match self.on_send.lock() {
            Ok(mut slot) => *slot = Some(Box::new(hook)),
            Err(poisoned) => *poisoned.into_inner() = Some(Box::new(hook)),
        }
    }

    pub(crate) fn sent(&self) -> Vec<SentRow> {
        match self.sent.lock() {
            Ok(rows) => rows.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}

impl Poster for ScriptPoster {
    fn post(&self, posted: &Posted) -> PostedOutcome {
        fire_hook(&self.on_send);
        push_sent(&self.sent, posted);
        match take_step(&self.steps) {
            Some(step) if step.fail => PostedOutcome::Fail(TransportFail::Unreachable),
            Some(step) => PostedOutcome::Http(HttpBytes {
                status: step.status,
                body: bytes::Bytes::from(step.body),
                retry_after: None,
            }),
            None => PostedOutcome::Fail(TransportFail::Unreachable),
        }
    }
}

fn fire_hook(slot: &Mutex<Option<Box<dyn Fn() + Send + Sync>>>) {
    match slot.lock() {
        Ok(hook) => {
            if let Some(hook) = hook.as_ref() {
                hook();
            }
        }
        Err(poisoned) => {
            if let Some(hook) = poisoned.into_inner().as_ref() {
                hook();
            }
        }
    }
}

fn push_sent(sent: &Mutex<Vec<SentRow>>, posted: &Posted) {
    let row = SentRow {
        url: posted.url.clone(),
        body: posted.body.clone(),
    };
    match sent.lock() {
        Ok(mut rows) => rows.push(row),
        Err(poisoned) => poisoned.into_inner().push(row),
    }
}

fn take_step(steps: &Mutex<Vec<ScriptStep>>) -> Option<ScriptStep> {
    match steps.lock() {
        Ok(mut steps) => steps.pop(),
        Err(poisoned) => poisoned.into_inner().pop(),
    }
}

#[cfg(test)]
pub(crate) fn ok_text(text: &str) -> ScriptStep {
    ScriptStep {
        status: 200,
        body: format!(
            "{{\"choices\":[{{\"message\":{{\"role\":\"assistant\",\"content\":{text:?}}},\"finish_reason\":\"stop\"}}]}}"
        ),
        fail: false,
    }
}

#[cfg(test)]
pub(crate) fn http_fail(status: u16, message: &str) -> ScriptStep {
    ScriptStep {
        status,
        body: format!("{{\"error\":{{\"message\":{message:?}}}}}"),
        fail: false,
    }
}

#[cfg(test)]
pub(crate) fn connect_fail() -> ScriptStep {
    ScriptStep {
        status: 0,
        body: String::new(),
        fail: true,
    }
}

pub(crate) fn session_with(poster: Arc<ScriptPoster>) -> crate::session::Session {
    let clock: Arc<dyn anybot_foundation::clock::Clock> = Arc::new(QuietClock);
    crate::session::Session::new(
        crate::client::Gateway::new(Arc::clone(&clock)).with_poster(poster),
        clock,
        0,
    )
}

struct QuietClock;

impl anybot_foundation::clock::Clock for QuietClock {
    fn now_ms(&self) -> anybot_foundation::clock::TimestampMs {
        anybot_foundation::clock::TimestampMs::from_millis(0)
    }

    fn sleep(
        &self,
        _span: std::time::Duration,
    ) -> anybot_foundation::clock::PinBoxFuture<'static, ()> {
        Box::pin(std::future::ready(()))
    }

    fn sleep_until(
        &self,
        _deadline: anybot_foundation::clock::TimestampMs,
    ) -> anybot_foundation::clock::PinBoxFuture<'static, ()> {
        Box::pin(std::future::ready(()))
    }
}

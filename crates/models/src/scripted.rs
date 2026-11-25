use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use bytes::Bytes;

use crate::http_wire::{HttpWire, WireBody, WireOpen, WireRequest, WireResponse};
use crate::transport::TransportFail;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScriptStep {
    pub status: u16,
    pub body: String,
    pub fail: bool,
    pub stall: bool,
    pub chunks: Vec<String>,
    pub cut_at: Option<usize>,
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

impl HttpWire for ScriptPoster {
    fn send<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WireResponse, TransportFail>> + Send + 'a>> {
        Box::pin(async move { self.reply(request).await })
    }

    fn open<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WireOpen, TransportFail>> + Send + 'a>> {
        Box::pin(async move { self.open_reply(request).await })
    }
}

impl ScriptPoster {
    async fn reply(&self, request: &WireRequest) -> Result<WireResponse, TransportFail> {
        let Some(step) = self.taken(request) else {
            return Err(TransportFail::Unreachable);
        };
        if step.fail {
            return Err(TransportFail::Unreachable);
        }
        if step.stall {
            return std::future::pending().await;
        }
        Ok(WireResponse {
            status: step.status,
            headers: Vec::new(),
            body: Bytes::from(step.body),
        })
    }

    async fn open_reply(&self, request: &WireRequest) -> Result<WireOpen, TransportFail> {
        let Some(step) = self.taken(request) else {
            return Err(TransportFail::Unreachable);
        };
        if step.fail {
            return Err(TransportFail::Unreachable);
        }
        if step.stall {
            return std::future::pending().await;
        }
        Ok(WireOpen {
            status: step.status,
            headers: Vec::new(),
            body: body_of(step),
        })
    }

    fn taken(&self, request: &WireRequest) -> Option<ScriptStep> {
        fire_hook(&self.on_send);
        push_sent(&self.sent, request);
        take_step(&self.steps)
    }
}

fn body_of(step: ScriptStep) -> WireBody {
    let pieces = if step.chunks.is_empty() && step.cut_at.is_none() {
        vec![Ok(Bytes::from(step.body))]
    } else {
        crate::scripted_body::chunk_pieces(&step)
    };
    WireBody::Stream(Box::pin(futures_util::stream::iter(pieces)))
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

fn push_sent(sent: &Mutex<Vec<SentRow>>, posted: &WireRequest) {
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

pub(crate) fn ok_text(text: &str) -> ScriptStep {
    script_step(
        200,
        format!(
            "{{\"choices\":[{{\"message\":{{\"role\":\"assistant\",\"content\":{text:?}}},\"finish_reason\":\"stop\"}}]}}"
        ),
        false,
    )
}

pub(crate) fn script_http(status: u16, body: &str) -> ScriptStep {
    script_step(status, body.to_owned(), false)
}

pub(crate) fn script_chunks(status: u16, chunks: &[String]) -> ScriptStep {
    let mut step = script_step(status, chunks.join(""), false);
    step.chunks = chunks.to_vec();
    step
}

pub(crate) fn script_cut(status: u16, body: &str, at: usize) -> ScriptStep {
    let mut step = script_step(status, body.to_owned(), false);
    step.cut_at = Some(at);
    step
}

fn script_step(status: u16, body: String, fail: bool) -> ScriptStep {
    ScriptStep {
        status,
        body,
        fail,
        stall: false,
        chunks: Vec::new(),
        cut_at: None,
    }
}

pub(crate) fn http_fail(status: u16, message: &str) -> ScriptStep {
    script_step(
        status,
        format!("{{\"error\":{{\"message\":{message:?}}}}}"),
        false,
    )
}

pub(crate) fn connect_fail() -> ScriptStep {
    script_step(0, String::new(), true)
}

pub(crate) fn session_with(poster: Arc<ScriptPoster>) -> crate::session::Session {
    let clock: Arc<dyn anybot_foundation::clock::Clock> = Arc::new(QuietClock);
    crate::session::Session::new(
        crate::client::Gateway::from_wire(poster, Arc::clone(&clock)),
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

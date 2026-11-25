use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use bytes::Bytes;

use crate::http_wire::{HttpWire, WireBody, WireOpen, WireRequest};
use crate::transport::TransportFail;

#[derive(Clone, Debug)]
pub(crate) struct ScriptStep {
    pub status: u16,
    pub body: String,
    pub fail: bool,
    pub stall: bool,
    pub chunks: Vec<String>,
    pub cut_at: Option<usize>,
    pub hold: Option<Arc<tokio::sync::Notify>>,
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
    fn open<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WireOpen, TransportFail>> + Send + 'a>> {
        Box::pin(async move { self.open_reply(request).await })
    }
}

impl ScriptPoster {
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
    let hold = step.hold.clone();
    let pieces = if step.chunks.is_empty() && step.cut_at.is_none() {
        vec![Ok(Bytes::from(step.body))]
    } else {
        crate::scripted_body::chunk_pieces(&step)
    };
    WireBody::Stream(Box::pin(async_stream(hold, pieces)))
}

fn async_stream(
    hold: Option<Arc<tokio::sync::Notify>>,
    pieces: Vec<Result<Bytes, TransportFail>>,
) -> impl futures_util::Stream<Item = Result<Bytes, TransportFail>> {
    futures_util::stream::unfold((hold, pieces.into_iter()), |state| async move {
        let (hold, mut pieces) = state;
        if let Some(wait) = hold {
            wait.notified().await;
            return pieces.next().map(|item| (item, (None, pieces)));
        }
        pieces.next().map(|item| (item, (None, pieces)))
    })
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
        hold: None,
    }
}

pub(crate) fn held(mut step: ScriptStep, hold: Arc<tokio::sync::Notify>) -> ScriptStep {
    step.hold = Some(hold);
    step
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
    session_policy(poster, open_policy())
}

pub(crate) fn session_policy(
    poster: Arc<ScriptPoster>,
    policy: crate::egress::DataPolicy,
) -> crate::session::Session {
    let clock: Arc<dyn anybot_foundation::clock::Clock> = Arc::new(QuietClock);
    crate::session::Session::new(
        crate::client::Gateway::from_wire(poster, Arc::clone(&clock)),
        policy,
        clock,
        0,
    )
}

fn open_policy() -> crate::egress::DataPolicy {
    crate::egress::DataPolicy::Cloud {
        allowed: vec![
            crate::types::ProviderKind::OpenAi,
            crate::types::ProviderKind::Anthropic,
            crate::types::ProviderKind::Custom,
            crate::types::ProviderKind::Ollama,
        ],
    }
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

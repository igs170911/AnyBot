use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::sse::{SSE_IDLE_RULE, SseOpen, SseRequest, open_sse_via, open_sse_with};

pub type OnReconnect = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct Target {
    pub base: String,
    pub path: &'static str,
    pub bearer: String,
    pub cookie: Option<String>,
    pub query: &'static str,
    pub origin: Option<std::net::SocketAddr>,
    pub proxy_port: Option<u16>,
    pub on_reconnect: Option<Arc<OnReconnect>>,
    pub stop_after_first: bool,
    pub use_first: Arc<AtomicBool>,
    pub first_origin: Option<std::net::SocketAddr>,
    pub first_port: Option<u16>,
}

impl Target {
    #[must_use]
    pub fn direct(base: &str, path: &'static str, bearer: &str, query: &'static str) -> Self {
        Target {
            base: base.to_owned(),
            path,
            bearer: bearer.to_owned(),
            cookie: None,
            query,
            origin: None,
            proxy_port: None,
            on_reconnect: None,
            stop_after_first: false,
            use_first: Arc::new(AtomicBool::new(false)),
            first_origin: None,
            first_port: None,
        }
    }

    #[must_use]
    pub fn proxied(
        origin: std::net::SocketAddr,
        path: &'static str,
        bearer: &str,
        query: &'static str,
        proxy_port: u16,
    ) -> Self {
        Target {
            base: String::new(),
            path,
            bearer: bearer.to_owned(),
            cookie: None,
            query,
            origin: Some(origin),
            proxy_port: Some(proxy_port),
            on_reconnect: None,
            stop_after_first: false,
            use_first: Arc::new(AtomicBool::new(false)),
            first_origin: None,
            first_port: None,
        }
    }

    pub fn use_direct(&self) {
        self.use_first.store(false, Ordering::SeqCst);
    }

    pub async fn open(&self) -> Result<SseOpen, String> {
        let mut spec = SseRequest::new(&self.base, self.path, &self.bearer, self.query);
        if let Some(cookie) = &self.cookie {
            spec = spec.with_cookie(cookie);
        }
        if self.use_first.load(Ordering::SeqCst)
            && let (Some(origin), Some(port)) = (self.first_origin, self.first_port)
        {
            return open_sse_via(origin, &spec, port).await;
        }
        match (self.origin, self.proxy_port) {
            (Some(origin), Some(port)) => open_sse_via(origin, &spec, port).await,
            _ => open_sse_with(&spec.with_origin(&self.base, "same-origin")).await,
        }
    }
}

pub struct Watched {
    pub stream: SseOpen,
    pub reconnects: usize,
    pub arrived: usize,
}

impl Watched {
    pub fn close(self) {}
}

#[must_use]
fn silent_stream() -> SseOpen {
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    drop(tx);
    SseOpen::silent(rx)
}

enum Step {
    More,
    Done,
}

fn watched(stream: SseOpen, reconnects: usize, arrived: usize) -> Watched {
    Watched {
        stream,
        reconnects,
        arrived,
    }
}

fn bump(counter: &mut usize) -> Result<(), String> {
    let next = counter
        .checked_add(1)
        .ok_or("watch counter overflow".to_owned())?;
    *counter = next;
    Ok(())
}

fn on_block(target: &Target, arrived: &mut usize, reconnects: usize) -> Result<Step, String> {
    bump(arrived)?;
    if target.stop_after_first && reconnects > 0 {
        return Ok(Step::Done);
    }
    Ok(Step::More)
}

async fn on_quiet(
    target: &Target,
    stream: &mut SseOpen,
    arrived: &mut usize,
    reconnects: &mut usize,
) -> Result<Step, String> {
    stream.close();
    test_harness::sleep_real(Duration::from_millis(10)).await;
    bump(reconnects)?;
    if let Some(handler) = &target.on_reconnect {
        handler().await;
    }
    *stream = match target.open().await {
        Ok(opened) => opened,
        Err(_silent) => silent_stream(),
    };
    if !target.stop_after_first {
        return Ok(Step::More);
    }
    let probe = stream.recv_within(Duration::from_secs(4)).await;
    if probe.is_ok() {
        bump(arrived)?;
        return Ok(Step::Done);
    }
    Ok(Step::More)
}

pub async fn watch(target: &Target, span: Duration) -> Result<Watched, String> {
    let started = test_harness::realtime::instant_now();
    let mut reconnects = 0usize;
    let mut arrived = 0usize;
    let mut stream = match target.open().await {
        Ok(opened) => opened,
        Err(_silent) => silent_stream(),
    };
    loop {
        let left = span.saturating_sub(started.elapsed());
        if left.is_zero() {
            return Ok(watched(stream, reconnects, arrived));
        }
        let quiet_span = SSE_IDLE_RULE.min(left);
        let step = match stream.recv_within(quiet_span).await {
            Ok(_block) => on_block(target, &mut arrived, reconnects)?,
            Err(_quiet) => on_quiet(target, &mut stream, &mut arrived, &mut reconnects).await?,
        };
        if matches!(step, Step::Done) {
            return Ok(watched(stream, reconnects, arrived));
        }
    }
}

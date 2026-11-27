use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use anybot_foundation::Clock;
use tower::{Layer, Service};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub(crate) struct ConnectTimedOut;

impl std::fmt::Display for ConnectTimedOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("connect phase timed out")
    }
}

impl std::error::Error for ConnectTimedOut {}

pub(crate) struct ConnectGate {
    connected: tokio::sync::watch::Sender<bool>,
    timed_out: AtomicBool,
}

impl ConnectGate {
    pub(crate) fn new() -> Arc<Self> {
        let (connected, _watcher) = tokio::sync::watch::channel(false);
        Arc::new(ConnectGate {
            connected,
            timed_out: AtomicBool::new(false),
        })
    }

    pub(crate) async fn wait_connected(&self) {
        let mut watcher = self.connected.subscribe();
        let seen = watcher.wait_for(|reached| *reached).await;
        drop(seen);
    }

    pub(crate) fn timed_out(&self) -> bool {
        self.timed_out.load(Ordering::SeqCst)
    }
}

#[derive(Clone)]
pub(crate) struct GateLayer {
    pub gate: Arc<ConnectGate>,
    pub clock: Arc<dyn Clock>,
    pub limit: Duration,
}

#[derive(Clone)]
pub(crate) struct GateService<S> {
    inner: S,
    layer: GateLayer,
}

impl<S> Layer<S> for GateLayer {
    type Service = GateService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GateService {
            inner,
            layer: self.clone(),
        }
    }
}

impl<S, R> Service<R> for GateService<S>
where
    S: Service<R, Error = BoxError>,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
{
    type Response = S::Response;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, BoxError>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), BoxError>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: R) -> Self::Future {
        let connecting = self.inner.call(request);
        let layer = self.layer.clone();
        Box::pin(async move {
            let expired = layer.clock.sleep(layer.limit);
            tokio::select! {
                biased;
                connected = connecting => {
                    if connected.is_ok() {
                        layer.gate.connected.send_replace(true);
                    }
                    connected
                }
                () = expired => {
                    layer.gate.timed_out.store(true, Ordering::SeqCst);
                    Err(Box::new(ConnectTimedOut) as BoxError)
                }
            }
        })
    }
}

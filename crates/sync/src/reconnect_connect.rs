use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use anybot_foundation::{Clock, TimestampMs, timeout};
use tower::{Layer, Service};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectPhaseTimeout {
    pub limit: Duration,
}

impl std::fmt::Display for ConnectPhaseTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "connect phase did not finish within {} ms",
            self.limit.as_millis()
        )
    }
}

impl std::error::Error for ConnectPhaseTimeout {}

#[must_use]
pub fn is_connect_phase_timeout(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(err) = current {
        if err.is::<ConnectPhaseTimeout>() {
            return true;
        }
        current = err.source();
    }
    false
}

#[derive(Debug, Default)]
pub struct ConnectProbe {
    last_connected_ms: AtomicU64,
    ever: std::sync::atomic::AtomicBool,
}

impl ConnectProbe {
    pub fn mark(&self, at: TimestampMs) {
        self.last_connected_ms.store(at.millis(), Ordering::SeqCst);
        self.ever.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn last(&self) -> Option<TimestampMs> {
        if !self.ever.load(Ordering::SeqCst) {
            return None;
        }
        Some(TimestampMs::from_millis(
            self.last_connected_ms.load(Ordering::SeqCst),
        ))
    }
}

#[derive(Clone)]
pub struct ConnectPhaseLayer {
    clock: Arc<dyn Clock>,
    limit: Duration,
    probe: Arc<ConnectProbe>,
}

impl ConnectPhaseLayer {
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>, limit: Duration, probe: Arc<ConnectProbe>) -> Self {
        ConnectPhaseLayer {
            clock,
            limit,
            probe,
        }
    }
}

impl<S> Layer<S> for ConnectPhaseLayer {
    type Service = ConnectPhase<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ConnectPhase {
            inner,
            clock: Arc::clone(&self.clock),
            limit: self.limit,
            probe: Arc::clone(&self.probe),
        }
    }
}

#[derive(Clone)]
pub struct ConnectPhase<S> {
    inner: S,
    clock: Arc<dyn Clock>,
    limit: Duration,
    probe: Arc<ConnectProbe>,
}

type GuardedFuture<T> = Pin<Box<dyn Future<Output = Result<T, BoxError>> + Send>>;

impl<S, R> Service<R> for ConnectPhase<S>
where
    S: Service<R, Error = BoxError>,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
{
    type Response = S::Response;
    type Error = BoxError;
    type Future = GuardedFuture<S::Response>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), BoxError>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: R) -> Self::Future {
        let connecting = self.inner.call(request);
        let guarded = timeout(self.clock.as_ref(), self.limit, connecting);
        let limit = self.limit;
        let clock = Arc::clone(&self.clock);
        let probe = Arc::clone(&self.probe);
        Box::pin(async move {
            match guarded.await {
                Ok(Ok(connection)) => {
                    probe.mark(clock.now_ms());
                    Ok(connection)
                }
                Ok(Err(error)) => Err(error),
                Err(_expired) => Err(Box::new(ConnectPhaseTimeout { limit }) as BoxError),
            }
        })
    }
}

#[cfg(test)]
#[path = "reconnect_connect_tests.rs"]
mod tests;

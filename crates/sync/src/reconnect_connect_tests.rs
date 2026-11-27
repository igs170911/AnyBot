use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use test_harness::{FakeClock, timeout_real};
use tower::{Layer, Service, ServiceExt};

use super::*;

#[derive(Clone)]
struct NeverConnects;

impl Service<u8> for NeverConnects {
    type Response = u8;
    type Error = BoxError;
    type Future = std::future::Pending<Result<u8, BoxError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), BoxError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _request: u8) -> Self::Future {
        std::future::pending()
    }
}

#[derive(Clone)]
struct ConnectsAfter {
    clock: Arc<FakeClock>,
    wait: Duration,
}

impl Service<u8> for ConnectsAfter {
    type Response = u8;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<u8, BoxError>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), BoxError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: u8) -> Self::Future {
        let sleeping = test_harness::sleep_for(&self.clock, self.wait);
        Box::pin(async move {
            sleeping.await;
            Ok(request)
        })
    }
}

fn layer(clock: &Arc<FakeClock>) -> (ConnectPhaseLayer, Arc<ConnectProbe>) {
    let probe = Arc::new(ConnectProbe::default());
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    let layer = ConnectPhaseLayer::new(shared, Duration::from_secs(5), Arc::clone(&probe));
    (layer, probe)
}

#[tokio::test]
async fn a_connect_that_never_finishes_fails_at_five_fake_seconds_and_not_before() {
    let clock = FakeClock::new(1_000_000);
    let (layer, probe) = layer(&clock);
    let mut service = layer.layer(NeverConnects);
    let call = service.ready().await.unwrap().call(1);
    let handle = tokio::spawn(call);
    test_harness::sleep_real(Duration::from_millis(200)).await;
    let _woken = test_harness::advance_clock(&clock, Duration::from_millis(4_999));
    test_harness::sleep_real(Duration::from_millis(200)).await;
    assert!(!handle.is_finished(), "still inside the five seconds");
    let _woken = test_harness::advance_clock(&clock, Duration::from_millis(1));
    let outcome = timeout_real(handle, Duration::from_secs(5)).await.unwrap();
    let error = outcome.unwrap().unwrap_err();
    assert!(is_connect_phase_timeout(error.as_ref()));
    assert_eq!(probe.last(), None);
}

#[tokio::test]
async fn a_connect_that_finishes_in_time_passes_and_marks_the_probe() {
    let clock = FakeClock::new(1_000_000);
    let (layer, probe) = layer(&clock);
    let inner = ConnectsAfter {
        clock: Arc::clone(&clock),
        wait: Duration::from_secs(3),
    };
    let mut service = layer.layer(inner);
    let call = service.ready().await.unwrap().call(7);
    let handle = tokio::spawn(call);
    test_harness::sleep_real(Duration::from_millis(200)).await;
    let _woken = test_harness::advance_clock(&clock, Duration::from_secs(3));
    let outcome = timeout_real(handle, Duration::from_secs(5)).await.unwrap();
    assert_eq!(outcome.unwrap().unwrap(), 7);
    assert_eq!(probe.last(), Some(TimestampMs::from_millis(1_003_000)));
}

#[test]
fn only_the_connect_timeout_error_is_recognised() {
    let other = std::io::Error::other("nope");
    assert!(!is_connect_phase_timeout(&other));
    let timeout = ConnectPhaseTimeout {
        limit: Duration::from_secs(5),
    };
    assert!(is_connect_phase_timeout(&timeout));
}

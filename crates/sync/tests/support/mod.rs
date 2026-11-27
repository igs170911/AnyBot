use std::sync::Arc;
use std::time::Duration;

use anybot_app_test::http::{RequestSpec, post};
use anybot_app_test::serve::{Served, serve_start_with};
use anybot_domain::version_release::ReleaseTable;
use anybot_foundation::{Clock, Secret, TrustedCerts};
use anybot_sync::heartbeat_keepalive::RequestActivity;
use anybot_sync::reconnect_http::{SyncHttp, build_sync_http};
use anybot_sync::sse_session::{SseEnd, SseSignal, SseTarget, run_sse_session};
use serde_json::{Value, json};
use test_harness::{FakeClock, advance_clock, sleep_real, timeout_real};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(crate) const CLIENT_VERSION: &str = "0.1.0";

pub(crate) struct Rig {
    pub(crate) served: Served,
    pub(crate) base: String,
    pub(crate) token: String,
}

pub(crate) async fn rig_with(name: &str, table: &ReleaseTable) -> Result<Rig, String> {
    let served = serve_start_with(name, table).await?;
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(&served).await?;
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-sync-login");
    let mut walker = anybot_app_test::clockwalk::ClockWalker::start(&served.clock);
    let answer = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    let token = answer?.bearer()?;
    Ok(Rig {
        served,
        base,
        token,
    })
}

pub(crate) async fn rig(name: &str) -> Result<Rig, String> {
    rig_with(name, &ReleaseTable::empty()).await
}

pub(crate) fn target(base: &str, token: &str) -> SseTarget {
    SseTarget {
        base_url: base.to_owned(),
        credential: Secret::from_untrusted(token.to_owned()),
        client_version: CLIENT_VERSION.to_owned(),
    }
}

pub(crate) fn http_on(
    clock: &Arc<FakeClock>,
    proxy: Option<&str>,
) -> Result<Arc<SyncHttp>, String> {
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    let built = build_sync_http(&TrustedCerts::empty(), proxy, shared)?;
    Ok(Arc::new(built))
}

pub(crate) struct Running {
    pub(crate) handle: JoinHandle<SseEnd>,
    pub(crate) signals: mpsc::Receiver<SseSignal>,
}

pub(crate) fn start_session(
    http: &Arc<SyncHttp>,
    clock: &Arc<FakeClock>,
    aim: SseTarget,
) -> Running {
    let (sender, signals) = mpsc::channel(64);
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    let activity = Arc::new(RequestActivity::new(Arc::clone(&shared)));
    let owned_http = Arc::clone(http);
    let owned_activity = Arc::clone(&activity);
    let handle = tokio::spawn(async move {
        run_sse_session(&owned_http, shared, &aim, &owned_activity, &sender).await
    });
    Running { handle, signals }
}

pub(crate) async fn next_signal(running: &mut Running) -> Result<SseSignal, String> {
    let got = timeout_real(running.signals.recv(), Duration::from_secs(20))
        .await
        .map_err(|_| "no signal within 20 real seconds".to_owned())?;
    got.ok_or_else(|| "signal channel closed".to_owned())
}

pub(crate) async fn settle() {
    sleep_real(Duration::from_millis(250)).await;
}

pub(crate) async fn walk(
    clock: &Arc<FakeClock>,
    total: Duration,
    step: Duration,
) -> Result<(), String> {
    let total_ms = u64::try_from(total.as_millis()).map_err(|e| e.to_string())?;
    let step_ms = u64::try_from(step.as_millis()).map_err(|e| e.to_string())?;
    let rounds = total_ms
        .checked_div(step_ms)
        .ok_or("step must not be zero".to_owned())?;
    for _round in 0..rounds {
        let _woken = advance_clock(clock, step);
        sleep_real(Duration::from_millis(20)).await;
    }
    Ok(())
}

pub(crate) async fn end_of(running: Running) -> Result<SseEnd, String> {
    timeout_real(running.handle, Duration::from_secs(20))
        .await
        .map_err(|_| "session did not end within 20 real seconds".to_owned())?
        .map_err(|e| e.to_string())
}

pub(crate) fn item(device: &str, text: &str) -> Value {
    json!({
        "device_id": device,
        "kind": "message",
        "content": {"text": text},
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    })
}

pub(crate) async fn append_one(rig: &Rig, key: &str, device: &str) -> Result<(), String> {
    let body = json!({"stream": "account", "items": [item(device, "x")]}).to_string();
    let spec = RequestSpec::new(&body)
        .with_key(key)
        .with_bearer(&rig.token);
    let answer = post(&rig.base, "/v1/streams/append", &spec).await?;
    if answer.status != 200 {
        return Err(format!("append {key}: {} {}", answer.status, answer.json));
    }
    Ok(())
}

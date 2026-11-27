use anybot_app_test::http::{RequestSpec, post};
use anybot_app_test::serve::{Served, serve_start_with};
use anybot_domain::version_release::ReleaseTable;
use anybot_sync::sse_session::SseSignal;
use serde_json::{Value, json};
use std::time::Duration;
use test_harness::timeout_real;

use crate::support::Running;

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

pub(crate) async fn next_signal(running: &mut Running) -> Result<SseSignal, String> {
    let got = timeout_real(running.signals.recv(), Duration::from_secs(20))
        .await
        .map_err(|_| "no signal within 20 real seconds".to_owned())?;
    got.ok_or_else(|| "signal channel closed".to_owned())
}

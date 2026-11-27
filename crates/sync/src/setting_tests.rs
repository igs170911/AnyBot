use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::Clock;
use serde_json::Value;
use test_harness::{FakeClock, sleep_real, test_dir};

use crate::client::SyncClient;
use crate::fake_wire::{FakeWire, page_reply, timed};
use crate::random::RandomSource;
use crate::wire::Wire;

struct Fixed;

impl RandomSource for Fixed {
    fn next_u64(&self) -> u64 {
        0
    }
}

const START_MS: u64 = 1_700_000_000_000;
const FIELD: &str = "accent_color";

async fn start(
    name: &str,
    clock: &Arc<FakeClock>,
    wire: &Arc<FakeWire>,
) -> Result<SyncClient, String> {
    let dir = test_dir(name)?;
    let clock_dyn: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    let wire_dyn: Arc<dyn Wire> = Arc::<FakeWire>::clone(wire);
    SyncClient::start_with_wire(&dir, wire_dyn, clock_dyn, Arc::new(Fixed))
        .await
        .map_err(|e| e.to_string())
}

async fn settle(clock: &Arc<FakeClock>) {
    for _round in 0..100 {
        sleep_real(Duration::from_millis(5)).await;
        let _woken = clock.advance(Duration::from_millis(100));
    }
}

fn first_setting(wire: &FakeWire) -> Option<Value> {
    wire.posted()
        .iter()
        .filter_map(|body| body.get("items").and_then(Value::as_array))
        .flatten()
        .next()
        .cloned()
}

#[tokio::test]
async fn a_setting_change_is_held_while_no_clock_offset_was_measured_this_connection()
-> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![]);
    let client = start("setting_held", &clock, &wire).await?;
    client
        .change_setting(FIELD, None, "green")
        .await
        .map_err(|e| e.to_string())?;
    settle(&clock).await;
    assert!(wire.posted().is_empty(), "posted: {:?}", wire.posted());
    let rows = client.outbox().await.map_err(|e| e.to_string())?;
    assert_eq!(rows.len(), 1);
    assert!(rows.iter().all(|row| row.attempted_at_ms.is_none()));
    let shown = client.settings().await.map_err(|e| e.to_string())?;
    assert_eq!(shown.len(), 1);
    assert_eq!(shown.first().map(|s| s.value.as_str()), Some("green"));
    Ok(())
}

#[tokio::test]
async fn a_setting_change_leaves_with_the_offset_measured_from_the_server_time_header()
-> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![]);
    let server_now = i64::try_from(START_MS).map_err(|e| e.to_string())? + 7_000;
    wire.on_get(
        "account",
        vec![Ok(timed(page_reply("account", &[], false, 0), server_now))],
    );
    let client = start("setting_offset", &clock, &wire).await?;
    settle(&clock).await;
    let changed_at = i64::try_from(clock.now_ms()).map_err(|e| e.to_string())?;
    client
        .change_setting(FIELD, None, "green")
        .await
        .map_err(|e| e.to_string())?;
    settle(&clock).await;
    let sent = first_setting(&wire).ok_or("nothing was sent")?;
    let content = sent.get("content").ok_or("no content")?;
    assert_eq!(
        content.get("offset_ms").and_then(Value::as_i64),
        Some(7_000)
    );
    assert_eq!(content.get("value").and_then(Value::as_str), Some("green"));
    let changed = content.get("changed_at_ms").and_then(Value::as_i64);
    assert_eq!(changed, Some(changed_at));
    Ok(())
}

#[tokio::test]
async fn a_setting_unknown_to_this_version_is_kept_and_merged() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![]);
    let changed = i64::try_from(START_MS).map_err(|e| e.to_string())?;
    let record = serde_json::json!({
        "seq": 1,
        "device_id": "0190aaaa-bbbb-7ccc-8ddd-000000000001",
        "kind": "setting",
        "content": {
            "field": "future_flag",
            "item": null,
            "value": "on",
            "changed_at_ms": changed,
            "offset_ms": 0,
        },
        "created_at_ms": changed,
        "offline_sent": false,
    });
    wire.on_get(
        "account",
        vec![Ok(page_reply("account", &[record], false, 1))],
    );
    let client = start("setting_unknown", &clock, &wire).await?;
    let mut shown = Vec::new();
    for _round in 0..200 {
        shown = client.settings().await.map_err(|e| e.to_string())?;
        if !shown.is_empty() {
            break;
        }
        sleep_real(Duration::from_millis(5)).await;
    }
    assert_eq!(shown.len(), 1);
    assert_eq!(shown.first().map(|s| s.field.as_str()), Some("future_flag"));
    assert_eq!(shown.first().map(|s| s.value.as_str()), Some("on"));
    Ok(())
}

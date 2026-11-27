use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use std::time::Duration;

use serde_json::Value;

use anybot_app_test::ClockWalker;
use anybot_app_test::serve::{Served, serve_start};
use anybot_server_test::computers_support::{api_get, computer_login, computer_rows, row_of};
use anybot_server_test::make_admin;

struct Listed {
    online: bool,
    at_ms: Option<i64>,
    reason: Option<String>,
}

async fn listed(served: &Served, phone: &str, device_id: &str) -> Result<Listed, String> {
    let rows = computer_rows(served, phone).await?;
    let row = row_of(&rows, device_id).ok_or("computer missing from the list")?;
    Ok(Listed {
        online: row
            .get("online")
            .and_then(Value::as_bool)
            .ok_or("no online flag")?,
        at_ms: row.get("offline_at_ms").and_then(Value::as_i64),
        reason: row
            .get("offline_reason")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

fn clock_ms(served: &Served) -> Result<i64, String> {
    i64::try_from(served.clock.now_ms()).map_err(|e| e.to_string())
}

async fn signal_after_quiet_gap(served: &Served, bearer: &str) -> Result<i64, String> {
    let mut walker = ClockWalker::manual(&served.clock);
    walker.step(Duration::from_secs(3)).await?;
    let signal = api_get(served, bearer, "/v1/computers").await?;
    assert_eq!(signal.status, 200, "{}", signal.json);
    clock_ms(served)
}

#[tokio::test]
async fn t_computers_online_1() -> Result<(), String> {
    let mut served = serve_start("t_computers_online_1").await?;
    let phone = make_admin(&served).await?;
    let computer = computer_login(&served, "B", "k-b").await?;
    let signal_ms = signal_after_quiet_gap(&served, &computer.bearer).await?;
    let mut walker = ClockWalker::manual(&served.clock);
    walker.step(Duration::from_secs(59)).await?;
    let early = listed(&served, &phone, &computer.device_id).await?;
    assert!(early.online, "B was offline after 59 seconds");
    assert_eq!(early.at_ms, None);
    walker.step(Duration::from_secs(2)).await?;
    let late = listed(&served, &phone, &computer.device_id).await?;
    assert!(!late.online, "B still online after 61 seconds");
    assert_eq!(late.at_ms, Some(signal_ms));
    assert_eq!(late.reason.as_deref(), Some("computers-offline-no-signal"));
    served.stop_server().await?;
    served.db.stop().await
}

#[tokio::test]
async fn t_computers_online_2() -> Result<(), String> {
    let mut served = serve_start("t_computers_online_2").await?;
    let phone = make_admin(&served).await?;
    let computer = computer_login(&served, "B", "k-b").await?;
    let signal_ms = signal_after_quiet_gap(&served, &computer.bearer).await?;
    served.stop_server().await?;
    let mut walker = ClockWalker::manual(&served.clock);
    walker.step(Duration::from_secs(30)).await?;
    served.restart_server().await?;
    walker.step(Duration::from_secs(59)).await?;
    let early = listed(&served, &phone, &computer.device_id).await?;
    assert!(early.online, "B was offline 59 seconds after the restart");
    walker.step(Duration::from_secs(2)).await?;
    let late = listed(&served, &phone, &computer.device_id).await?;
    assert!(!late.online, "B still online 61 seconds after the restart");
    assert_eq!(late.at_ms, Some(signal_ms));
    assert_eq!(late.reason, None);
    served.stop().await
}

#[tokio::test]
async fn t_computers_online_7() -> Result<(), String> {
    let mut served = serve_start("t_computers_online_7").await?;
    let phone = make_admin(&served).await?;
    let computer = computer_login(&served, "B", "k-b").await?;
    let signal_ms = signal_after_quiet_gap(&served, &computer.bearer).await?;
    let mut walker = ClockWalker::manual(&served.clock);
    walker.step(Duration::from_secs(61)).await?;
    let before = listed(&served, &phone, &computer.device_id).await?;
    assert!(!before.online);
    assert_eq!(before.at_ms, Some(signal_ms));
    assert_eq!(before.reason.as_deref(), Some("computers-offline-no-signal"));
    served.stop_server().await?;
    walker.step(Duration::from_secs(120)).await?;
    served.restart_server().await?;
    let after = listed(&served, &phone, &computer.device_id).await?;
    assert!(!after.online);
    assert_eq!(after.at_ms, before.at_ms);
    assert_eq!(after.reason, before.reason);
    served.stop().await
}

#[tokio::test]
async fn computers_online_killed_server_loses_at_most_the_write_throttle() -> Result<(), String> {
    let mut served = serve_start("computers_online_killed_server").await?;
    let phone = make_admin(&served).await?;
    let computer = computer_login(&served, "B", "k-b").await?;
    let signal_ms = signal_after_quiet_gap(&served, &computer.bearer).await?;
    served.kill_server().await?;
    served.restart_server().await?;
    let mut walker = ClockWalker::manual(&served.clock);
    walker.step(Duration::from_secs(61)).await?;
    let after = listed(&served, &phone, &computer.device_id).await?;
    assert!(!after.online);
    let shown = after.at_ms.ok_or("no offline time")?;
    assert!(shown < signal_ms, "the unwritten signal survived a kill");
    assert!(signal_ms.saturating_sub(shown) <= 10_000);
    assert_eq!(after.reason, None);
    served.stop().await
}

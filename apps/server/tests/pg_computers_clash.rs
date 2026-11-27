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
use anybot_app_test::serve::serve_start;
use anybot_server_test::computers_support::{
    api_get, computer_login, computer_rows, name_of, row_of,
};
use anybot_server_test::make_admin;

const WORK_MACHINE: &str = "\u{5de5}\u{4f5c}\u{6a5f}";
const WORK_MACHINE_SECOND: &str = "\u{5de5}\u{4f5c}\u{6a5f} (2)";

#[tokio::test]
async fn t_computers_online_4() -> Result<(), String> {
    let served = serve_start("t_computers_online_4").await?;
    let phone = make_admin(&served).await?;
    let first = computer_login(&served, WORK_MACHINE, "k-a").await?;
    let second = computer_login(&served, WORK_MACHINE, "k-b").await?;
    let rows = computer_rows(&served, &phone).await?;
    let new_row = row_of(&rows, &second.device_id).ok_or("new computer missing")?;
    assert_eq!(name_of(new_row), Some(WORK_MACHINE_SECOND));
    let old_row = row_of(&rows, &first.device_id).ok_or("first computer was dropped")?;
    assert_eq!(name_of(old_row), Some(WORK_MACHINE));
    assert_eq!(old_row.get("online").and_then(Value::as_bool), Some(true));
    let still = api_get(&served, &first.bearer, "/v1/computers").await?;
    assert_eq!(
        still.status, 200,
        "first computer disconnected: {}",
        still.json
    );
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_computers_online_5() -> Result<(), String> {
    let served = serve_start("t_computers_online_5").await?;
    let phone = make_admin(&served).await?;
    let first = computer_login(&served, WORK_MACHINE, "k-a").await?;
    let mut walker = ClockWalker::manual(&served.clock);
    walker.step(Duration::from_secs(61)).await?;
    let before = computer_rows(&served, &phone).await?;
    let old_row = row_of(&before, &first.device_id).ok_or("first computer missing")?;
    assert_eq!(old_row.get("online").and_then(Value::as_bool), Some(false));
    let second = computer_login(&served, WORK_MACHINE, "k-b").await?;
    let rows = computer_rows(&served, &phone).await?;
    assert_eq!(rows.len(), 1, "list: {rows:?}");
    let only = rows.first().ok_or("empty list")?;
    assert_eq!(name_of(only), Some(WORK_MACHINE));
    assert_eq!(
        only.get("device_id").and_then(Value::as_str),
        Some(second.device_id.as_str())
    );
    let gone = api_get(&served, &first.bearer, "/v1/computers").await?;
    assert_eq!(gone.status, 401, "{}", gone.json);
    assert_eq!(gone.code().as_deref(), Some("computer-removed"));
    served.stop().await?;
    Ok(())
}

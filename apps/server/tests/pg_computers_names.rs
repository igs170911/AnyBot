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

use serde_json::{Value, json};

use anybot_app_test::log_capture::capture_server_logs;
use anybot_app_test::serve::serve_start;
use anybot_server_test::computers_support::{
    api_post, computer_login, computer_rows, name_of, row_of,
};
use anybot_server_test::make_admin;

async fn rename(
    served: &anybot_app_test::serve::Served,
    bearer: &str,
    device_id: &str,
    name: &str,
    key: &str,
) -> Result<anybot_app_test::http::ResponseSpec, String> {
    let body = json!({"device_id": device_id, "name": name});
    api_post(served, bearer, "/v1/computers/rename", &body, key).await
}

#[tokio::test]
async fn t_computers_n8_2() -> Result<(), String> {
    let logs = capture_server_logs()?;
    let served = serve_start("t_computers_n8_2").await?;
    let phone = make_admin(&served).await?;
    let computer = computer_login(&served, "A", "k-a").await?;
    let too_long = "x".repeat(61);
    let refused = rename(
        &served,
        &computer.bearer,
        &computer.device_id,
        &too_long,
        "k-1",
    )
    .await?;
    assert_eq!(refused.status, 400, "{}", refused.json);
    assert_eq!(refused.code().as_deref(), Some("computer-name-too-long"));
    let split = "\u{7532}\n\u{4e59}";
    let accepted = rename(&served, &computer.bearer, &computer.device_id, split, "k-2").await?;
    assert_eq!(accepted.status, 200, "{}", accepted.json);
    let rows = computer_rows(&served, &phone).await?;
    let row = row_of(&rows, &computer.device_id).ok_or("computer missing")?;
    assert_eq!(name_of(row), Some("\u{7532} \u{4e59}"));
    let lines = logs.lines();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("name=\u{7532} \u{4e59}")),
        "no log line with the cleaned name: {lines:?}"
    );
    assert!(
        lines.iter().all(|line| !line.starts_with('\u{4e59}')),
        "the name was broken across log lines: {lines:?}"
    );
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_computers_n8_3() -> Result<(), String> {
    let served = serve_start("t_computers_n8_3").await?;
    make_admin(&served).await?;
    let first = computer_login(&served, "A", "k-a").await?;
    computer_login(&served, "B", "k-b").await?;
    let refused = rename(&served, &first.bearer, &first.device_id, "B", "k-1").await?;
    assert_eq!(refused.status, 409, "{}", refused.json);
    assert_eq!(refused.code().as_deref(), Some("computer-name-taken"));
    let named = refused
        .json
        .get("params")
        .and_then(|params| params.get("name"))
        .and_then(Value::as_str);
    assert_eq!(named, Some("B"));
    served.stop().await?;
    Ok(())
}

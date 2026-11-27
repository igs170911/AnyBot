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

use anybot_app_test::serve::serve_start;
use anybot_server_test::computers_support::{api_post, computer_login, computer_rows, row_of};
use anybot_server_test::make_admin;

async fn delegate_flag(
    served: &anybot_app_test::serve::Served,
    phone: &str,
    device_id: &str,
) -> Result<Option<bool>, String> {
    let rows = computer_rows(served, phone).await?;
    Ok(row_of(&rows, device_id)
        .and_then(|row| row.get("delegate_enabled"))
        .and_then(Value::as_bool))
}

#[tokio::test]
async fn t_computers_n2_5() -> Result<(), String> {
    let served = serve_start("t_computers_n2_5").await?;
    let phone = make_admin(&served).await?;
    let computer_a = computer_login(&served, "A", "k-a").await?;
    let computer_b = computer_login(&served, "B", "k-b").await?;
    let body = json!({"device_id": computer_a.device_id, "enabled": true});
    let path = "/v1/computers/delegate";
    let from_phone = api_post(&served, &phone, path, &body, "k-1").await?;
    assert_eq!(from_phone.status, 403, "{}", from_phone.json);
    let from_b = api_post(&served, &computer_b.bearer, path, &body, "k-2").await?;
    assert_eq!(from_b.status, 403, "{}", from_b.json);
    assert_eq!(
        delegate_flag(&served, &phone, &computer_a.device_id).await?,
        Some(false)
    );
    let from_a = api_post(&served, &computer_a.bearer, path, &body, "k-3").await?;
    assert_eq!(from_a.status, 200, "{}", from_a.json);
    assert_eq!(
        delegate_flag(&served, &phone, &computer_a.device_id).await?,
        Some(true)
    );
    served.stop().await?;
    Ok(())
}

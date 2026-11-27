use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json::json;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::serve::serve_start;
use anybot_server_test::computers_support::{
    RedeemAttempt, api_post, code_text, computer_login, computer_rows, make_code, name_of, row_of,
};
use anybot_server_test::make_admin;

#[tokio::test]
async fn a_paired_computer_is_refused_the_account_level_actions() -> Result<(), String> {
    let served = serve_start("paired_restrictions").await?;
    let phone = make_admin(&served).await?;
    let other = computer_login(&served, "A", "k-a").await?;
    let code = code_text(&served, &phone, "k-code").await?;
    let attempt = RedeemAttempt {
        code: &code,
        name: "B",
        key: "r-1",
        address: None,
    };
    let paired = anybot_server_test::computers_support::redeem(&served, &attempt).await?;
    assert_eq!(paired.status, 200, "{}", paired.json);
    let paired_key = paired.bearer()?;
    let paired_id = paired.field_str("device_id")?;
    let code_try = make_code(&served, &paired_key, "k-1").await?;
    assert_eq!(code_try.status, 403, "{}", code_try.json);
    assert_eq!(
        code_try.code().as_deref(),
        Some("computer-paired-forbidden")
    );
    let remove = json!({"device_id": other.device_id});
    let removal = api_post(&served, &paired_key, "/v1/computers/remove", &remove, "k-2").await?;
    assert_eq!(removal.status, 403, "{}", removal.json);
    assert_eq!(removal.code().as_deref(), Some("computer-paired-forbidden"));
    let sharing = json!({"device_id": paired_id, "enabled": true});
    let change = api_post(
        &served,
        &paired_key,
        "/v1/computers/delegate",
        &sharing,
        "k-3",
    )
    .await?;
    assert_eq!(change.status, 403, "{}", change.json);
    assert_eq!(change.code().as_deref(), Some("computer-paired-forbidden"));
    let rows = computer_rows(&served, &phone).await?;
    assert!(row_of(&rows, &other.device_id).is_some(), "A was removed");
    let own = row_of(&rows, &paired_id).ok_or("paired computer missing")?;
    assert_eq!(name_of(own), Some("B"));
    let own_exit = json!({"device_id": paired_id});
    let leave = api_post(
        &served,
        &paired_key,
        "/v1/computers/remove",
        &own_exit,
        "k-4",
    )
    .await?;
    assert_eq!(leave.status, 200, "{}", leave.json);
    served.stop().await?;
    Ok(())
}

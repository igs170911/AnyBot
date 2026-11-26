use anybot_server_test as _;
use serde_json::json;

use anybot_app_server as _;
use anybot_app_test as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::http::{FORWARDED_HEADER, RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::serve_start;
use anybot_server::KEY_TRUST_PROXY;

fn register_spec(account: &str, password: &str, invite: &str, key: &str) -> RequestSpec {
    RequestSpec::new(
        &json!({"account": account, "password": password, "invite": invite}).to_string(),
    )
    .with_key(key)
}

async fn register_once(
    base: String,
    account: String,
    invite: String,
    key: String,
    address: String,
) -> Result<ResponseSpec, String> {
    post(
        &base,
        "/v1/account/register",
        &register_spec(&account, "longenough", &invite, &key)
            .with_header(FORWARDED_HEADER, &address),
    )
    .await
}

#[tokio::test]
async fn concurrent_same_name_registration_rejects_the_loser() -> Result<(), String> {
    let mut served = serve_start("register_same_name").await?;
    served
        .set_config_and_restart(KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    let invite = served
        .started
        .new_invite
        .clone()
        .ok_or("fresh boot must draw an invite")?;
    let base = served.base_url();
    let mut tasks = Vec::new();
    for index in 0..8 {
        tasks.push(tokio::spawn(register_once(
            base.clone(),
            "sharedname".to_owned(),
            invite.clone(),
            format!("same-{index}"),
            format!("10.50.0.{index}"),
        )));
    }
    let mut created = 0usize;
    let mut taken = 0usize;
    for task in tasks {
        let answer = task.await.map_err(|e| e.to_string())??;
        match answer.status {
            201 => created = created.saturating_add(1),
            409 => {
                assert_eq!(answer.code().as_deref(), Some("account-exists"));
                taken = taken.saturating_add(1);
            }
            other => return Err(format!("status {other} body {}", answer.json)),
        }
    }
    assert_eq!(created, 1, "one registration must win");
    assert_eq!(taken, 7, "every loser must be told the name is taken");
    served.stop().await?;
    Ok(())
}

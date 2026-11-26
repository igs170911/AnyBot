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

use anybot_app_test::http::{
    FORWARDED_HEADER, LOGIN_STEP, RequestSpec, ResponseSpec, post, post_driven,
};
use anybot_app_test::serve::serve_start;
use anybot_server::KEY_TRUST_PROXY;
use std::time::Duration;

fn register_spec(account: &str, invite: &str, key: &str) -> RequestSpec {
    RequestSpec::new(
        &json!({"account": account, "password": "longenough", "invite": invite}).to_string(),
    )
    .with_key(key)
}

async fn admin_bearer(served: &anybot_app_test::serve::Served) -> Result<(String, String), String> {
    let invite = served
        .started
        .new_invite
        .clone()
        .ok_or("fresh boot must draw an invite")?;
    let created = post(
        &served.base_url(),
        "/v1/account/register",
        &register_spec("alice", &invite, "k-admin"),
    )
    .await?;
    assert_eq!(created.status, 201, "{}", created.json);
    let session = post_driven(
        &served.clock,
        &served.base_url(),
        "/v1/account/login",
        &RequestSpec::new(
            &json!({"account": "alice", "password": "longenough", "label": "desk", "credential": "bearer"}).to_string(),
        )
        .with_key("k-login"),
        LOGIN_STEP,
        Duration::from_secs(15),
    )
    .await?;
    assert_eq!(session.status, 200, "{}", session.json);
    Ok((invite, session.bearer()?))
}

async fn rotate(base: &str, bearer: &str, key: &str) -> Result<String, String> {
    let answer = post(
        base,
        "/v1/admin/invite/rotate",
        &RequestSpec::new("{\"ok\":true}")
            .with_bearer(bearer)
            .with_key(key),
    )
    .await?;
    if answer.status != 200 {
        return Err(format!("rotate {} {}", answer.status, answer.json));
    }
    answer.field_str("invite")
}

async fn register_old(base: String, old: String, index: usize) -> Result<ResponseSpec, String> {
    post(
        &base,
        "/v1/account/register",
        &register_spec(&format!("late{index}"), &old, &format!("late-{index}"))
            .with_header(FORWARDED_HEADER, &format!("10.60.0.{index}")),
    )
    .await
}

#[tokio::test]
async fn rotated_invite_cannot_register_once_more() -> Result<(), String> {
    let mut served = serve_start("invite_rotate_race").await?;
    served
        .set_config_and_restart(KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    let (old, bearer) = admin_bearer(&served).await?;
    let fresh = rotate(&served.base_url(), &bearer, "k-rotate").await?;
    assert_ne!(fresh, old, "rotation must change the code");
    let base = served.base_url();
    let mut tasks = Vec::new();
    for index in 0..6 {
        tasks.push(tokio::spawn(register_old(base.clone(), old.clone(), index)));
    }
    for task in tasks {
        let answer = task.await.map_err(|e| e.to_string())??;
        assert_eq!(answer.status, 403, "old code body {}", answer.json);
        assert_eq!(answer.code().as_deref(), Some("account-invite-invalid"));
    }
    let created = post(
        &base,
        "/v1/account/register",
        &register_spec("newbie", &fresh, "k-fresh"),
    )
    .await?;
    assert_eq!(created.status, 201, "new code {}", created.json);
    served.stop().await?;
    Ok(())
}
